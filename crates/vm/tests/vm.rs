//! Top-level testing of the VM.

mod util;

use essential_vm::{
    asm::{self, short::*, Op},
    types::solution::{Mutation, Solution},
    Access, BytecodeMapped, Gas, GasLimit, Vm,
};
use std::sync::Arc;
use util::*;

// A simple sanity test to check basic functionality.
#[test]
fn no_yield() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let spent = vm
        .exec_ops(
            ops,
            test_access().clone(),
            &State::EMPTY,
            op_gas_cost,
            GasLimit::UNLIMITED,
        )
        .unwrap();
    assert_eq!(spent, ops.iter().map(op_gas_cost).sum::<Gas>());
    assert_eq!(vm.pc, ops.len() - 1);
    assert_eq!(&vm.stack[..], &[42]);
}

// Test VM behaves as expected when continuing execution over more operations.
#[test]
fn continue_execution() {
    let mut vm = Vm::default();

    // Execute first contract of ops.
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let spent = vm
        .exec_ops(
            ops,
            test_access().clone(),
            &State::EMPTY,
            op_gas_cost,
            GasLimit::UNLIMITED,
        )
        .unwrap();
    assert_eq!(spent, ops.iter().map(op_gas_cost).sum::<Gas>());
    assert_eq!(vm.pc, ops.len() - 1);
    assert_eq!(&vm.stack[..], &[42]);

    // Continue executing from current state over the new ops.
    vm.pc = 0;
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Alu::Div.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    let spent = vm
        .exec_ops(
            ops,
            test_access().clone(),
            &State::EMPTY,
            &op_gas_cost,
            GasLimit::UNLIMITED,
        )
        .unwrap();
    assert_eq!(spent, ops.iter().map(op_gas_cost).sum::<Gas>());
    assert_eq!(vm.pc, ops.len() - 1);
    assert_eq!(&vm.stack[..], &[7]);
}

// Ensure basic programs evaluate to the same thing
#[test]
fn exec_method_behaviours_match() {
    // The operations of the test program.
    let ops: &[Op] = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
        asm::TotalControlFlow::Halt.into(),
    ];

    // Execute the ops using `exec_ops`.
    let mut vm_ops = Vm::default();
    let spent_ops = vm_ops
        .exec_ops(
            ops,
            test_access().clone(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .unwrap();

    // Execute the ops using `exec_ops`.
    let mut vm_sync_ops = Vm::default();
    let spent_sync_ops = vm_sync_ops
        .exec_ops(
            ops,
            test_access().clone(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .unwrap();
    assert_eq!(spent_ops, spent_sync_ops);
    assert_eq!(vm_ops, vm_sync_ops);

    // Execute the same ops but as mapped bytecode.
    let mapped: BytecodeMapped = ops.iter().copied().collect();
    let mut vm_bc = Vm::default();
    let spent_bc = vm_bc
        .exec_bytecode(
            &mapped,
            test_access().clone(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .unwrap();
    assert_eq!(spent_sync_ops, spent_bc);
    assert_eq!(vm_sync_ops, vm_bc);
}

// Emulate the process of reading pre state, applying mutations to produce
// post state, and checking the constraints afterwards.
#[test]
fn read_pre_post_state() {
    let predicate_addr = TEST_PREDICATE_ADDR;

    // In the pre-state, we have [Some(40), None, Some(42)].
    let pre_state = State::new(vec![(
        predicate_addr.contract.clone(),
        vec![(vec![0, 0, 0, 0], vec![40]), (vec![0, 0, 0, 2], vec![42])],
    )]);

    // The solutions that we're checking.
    let solutions = Arc::new(vec![Solution {
        predicate_to_solve: predicate_addr.clone(),
        predicate_data: vec![],
        // We have one mutation that contracts a missing value to 41.
        state_mutations: vec![Mutation {
            key: vec![0, 0, 0, 1],
            value: vec![41],
        }],
    }]);

    // The index of the solution associated with the predicate we're solving.
    let solution_index = 0;

    // Construct access to the necessary solution for the VM.
    let access = Access::new(solutions.clone(), solution_index);

    // A simple program that reads words directly to memory.
    let ops = &[
        PUSH(9), // index, len, index, len, index, len, val, val, val
        ALOC,
        PUSH(0), // Key0
        PUSH(0), // Key1
        PUSH(0), // Key2
        PUSH(0), // Key3
        PUSH(4), // Key length
        PUSH(3), // Num words
        PUSH(0), // Slot index
        KRNG,
    ];

    // Execute the program.
    let mut vm = Vm::default();
    vm.exec_ops(
        ops,
        access.clone(),
        &pre_state,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .unwrap();

    // Collect the memory.
    let pre_state_mem: Vec<_> = vm.memory.into();

    // Apply the state mutations to the state to produce the post state.
    let mut post_state = pre_state.clone();
    for solution in solutions.iter() {
        let contract_addr = &solution.predicate_to_solve.contract;
        for Mutation { key, value } in &solution.state_mutations {
            post_state.set(contract_addr.clone(), key, value.clone());
        }
    }

    // Execute the program with the post state.
    let mut vm = Vm::default();
    vm.exec_ops(ops, access, &post_state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .unwrap();

    // Collect the state slots.
    let post_state_mem: Vec<_> = vm.memory.into();

    // Memory should have been updated.
    assert_eq!(pre_state_mem, vec![6, 1, 7, 0, 7, 1, 40, 42, 0]);
    assert_eq!(post_state_mem, vec![6, 1, 7, 1, 8, 1, 40, 41, 42]);
}

#[test]
fn read_sync_state() {
    let predicate_addr = TEST_PREDICATE_ADDR;

    // In the pre-state, we have [Some(40), None, Some(42)].
    let pre_state = State::new(vec![(
        predicate_addr.contract.clone(),
        vec![(vec![0, 0, 0, 0], vec![40]), (vec![0, 0, 0, 2], vec![42])],
    )]);

    // The full solution set that we're checking.
    let solutions = Arc::new(vec![Solution {
        predicate_to_solve: predicate_addr.clone(),
        predicate_data: vec![],
        // We have one mutation that contracts a missing value to 41.
        state_mutations: vec![Mutation {
            key: vec![0, 0, 0, 1],
            value: vec![41],
        }],
    }]);

    // The index of the solution associated with the predicate we're solving.
    let solution_index = 0;

    // Construct access to the necessary solution for the VM.
    let access = Access::new(solutions.clone(), solution_index);

    // A simple program that reads words directly to memory.
    let ops = &[
        PUSH(9), // index, len, index, len, index, len, val, val, val
        ALOC,
        PUSH(0), // Key0
        PUSH(0), // Key1
        PUSH(0), // Key2
        PUSH(0), // Key3
        PUSH(4), // Key length
        PUSH(3), // Num words
        PUSH(0), // Slot index
        KRNG,
    ];

    // Execute the program.
    let mut vm = Vm::default();
    vm.exec_ops(
        ops,
        access.clone(),
        &pre_state,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .unwrap();

    // Collect the memory.
    let pre_state_mem: Vec<_> = vm.memory.into();

    // Apply the state mutations to the state to produce the post state.
    let mut post_state = pre_state.clone();
    for solution in solutions.iter() {
        let contract_addr = &solution.predicate_to_solve.contract;
        for Mutation { key, value } in &solution.state_mutations {
            post_state.set(contract_addr.clone(), key, value.clone());
        }
    }

    // Execute the program with the post state.
    let mut vm = Vm::default();
    vm.exec_ops(ops, access, &post_state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .unwrap();

    // Collect the state slots.
    let post_state_mem: Vec<_> = vm.memory.into();

    // Memory should have been updated.
    assert_eq!(pre_state_mem, vec![6, 1, 7, 0, 7, 1, 40, 42, 0]);
    assert_eq!(post_state_mem, vec![6, 1, 7, 1, 8, 1, 40, 41, 42]);
}
// Test that halt is not required to end the vm.
#[test]
fn test_halt() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    vm.exec_ops(
        ops,
        test_access().clone(),
        &State::EMPTY,
        op_gas_cost,
        GasLimit::UNLIMITED,
    )
    .unwrap();
    assert_eq!(&vm.stack[..], &[42]);
}
