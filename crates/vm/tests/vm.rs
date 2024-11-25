//! Top-level testing of the VM.

mod util;

use essential_vm::{
    asm::{self, short::*, Op},
    mut_keys_set,
    types::solution::{Mutation, Solution, SolutionData},
    Access, BytecodeMapped, Gas, GasLimit, Vm,
};
use util::*;

// A simple sanity test to check basic functionality.
#[tokio::test]
async fn no_yield() {
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
            *test_access(),
            &State::EMPTY,
            op_gas_cost,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
    assert_eq!(spent, ops.iter().map(op_gas_cost).sum::<Gas>());
    assert_eq!(vm.pc, ops.len() - 1);
    assert_eq!(&vm.stack[..], &[42]);
}

// Test that we get expected results when yielding due to gas limits.
#[tokio::test]
async fn yield_per_op() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    // Force the VM to yield after every op to test behaviour.
    let op_gas_cost = |_op: &_| GasLimit::DEFAULT_PER_YIELD;

    let state = State::EMPTY;
    let mut future = vm.exec_ops(
        ops,
        *test_access(),
        &state,
        &op_gas_cost,
        GasLimit::UNLIMITED,
    );

    // Test that we yield once per op before reaching `Halt`.
    let mut yield_count = 0;
    let spent = {
        let mut future = std::pin::pin!(future);
        loop {
            match futures::poll!(&mut future) {
                std::task::Poll::Pending => yield_count += 1,
                std::task::Poll::Ready(res) => break res.unwrap(),
            }
        }
    };

    assert_eq!(yield_count, ops.len() - 1);
    assert_eq!(spent, ops.iter().map(op_gas_cost).sum::<Gas>());
    assert_eq!(vm.pc, ops.len() - 1);
    assert_eq!(&vm.stack[..], &[42]);
}

// Test VM behaves as expected when continuing execution over more operations.
#[tokio::test]
async fn continue_execution() {
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
            *test_access(),
            &State::EMPTY,
            op_gas_cost,
            GasLimit::UNLIMITED,
        )
        .await
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
            *test_access(),
            &State::EMPTY,
            &op_gas_cost,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
    assert_eq!(spent, ops.iter().map(op_gas_cost).sum::<Gas>());
    assert_eq!(vm.pc, ops.len() - 1);
    assert_eq!(&vm.stack[..], &[7]);
}

// Ensure basic programs evaluate to the same thing
#[tokio::test]
async fn exec_method_behaviours_match() {
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
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();

    // Execute the same ops but as mapped bytecode.
    let mapped: BytecodeMapped = ops.iter().copied().collect();
    let mut vm_bc = Vm::default();
    let spent_bc = vm_bc
        .exec_bytecode(
            &mapped,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
    assert_eq!(spent_ops, spent_bc);
    assert_eq!(vm_ops, vm_bc);

    // Execute the same ops, but from a bytes iterator.
    let bc_iter = mapped.bytecode().iter().copied();
    let mut vm_bc_iter = Vm::default();
    let spent_bc_iter = vm_bc_iter
        .exec_bytecode_iter(
            bc_iter,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
    assert_eq!(spent_ops, spent_bc_iter);
    assert_eq!(vm_ops, vm_bc_iter);
}

// Emulate the process of reading pre state, applying mutations to produce
// post state, and checking the constraints afterwards.
#[tokio::test]
async fn read_pre_post_state() {
    let predicate_addr = TEST_PREDICATE_ADDR;

    // In the pre-state, we have [Some(40), None, Some(42)].
    let pre_state = State::new(vec![(
        predicate_addr.contract.clone(),
        vec![(vec![0, 0, 0, 0], vec![40]), (vec![0, 0, 0, 2], vec![42])],
    )]);

    // The full solution that we're checking.
    let solution = Solution {
        data: vec![SolutionData {
            predicate_to_solve: predicate_addr.clone(),
            decision_variables: vec![],
            // We have one mutation that contracts a missing value to 41.
            state_mutations: vec![Mutation {
                key: vec![0, 0, 0, 1],
                value: vec![41],
            }],
        }],
    };

    // The index of the solution data associated with the predicate we're solving.
    let predicate_index = 0;

    let mutable_keys = mut_keys_set(&solution, predicate_index);

    // Construct access to the necessary solution data for the VM.
    let access = Access::new(&solution, predicate_index, &mutable_keys);

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
    vm.exec_ops(ops, access, &pre_state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();

    // Collect the memory.
    let pre_state_mem: Vec<_> = vm.memory.into();

    // Apply the state mutations to the state to produce the post state.
    let mut post_state = pre_state.clone();
    for data in &solution.data {
        let contract_addr = &data.predicate_to_solve.contract;
        for Mutation { key, value } in &data.state_mutations {
            post_state.set(contract_addr.clone(), key, value.clone());
        }
    }

    // Execute the program with the post state.
    let mut vm = Vm::default();
    vm.exec_ops(ops, access, &post_state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();

    // Collect the state slots.
    let post_state_mem: Vec<_> = vm.memory.into();

    // Memory should have been updated.
    assert_eq!(pre_state_mem, vec![6, 1, 7, 0, 7, 1, 40, 42, 0]);
    assert_eq!(post_state_mem, vec![6, 1, 7, 1, 8, 1, 40, 41, 42]);
}

// Test that halt is not required to end the vm.
#[tokio::test]
async fn test_halt() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        op_gas_cost,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(&vm.stack[..], &[42]);
}
