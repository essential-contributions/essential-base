//! Top-level testing of the VM.

mod util;

use constraint::mut_keys_set;
use essential_state_read_vm::{
    asm::{self, Op},
    constraint,
    types::solution::{Mutation, Solution, SolutionData, StateMutation},
    Access, BytecodeMapped, Gas, GasLimit, SolutionAccess, StateSlots, Vm,
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
        asm::ControlFlow::Halt.into(),
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
        asm::ControlFlow::Halt.into(),
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

    // Execute first set of ops.
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
        asm::ControlFlow::Halt.into(),
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
        asm::ControlFlow::Halt.into(),
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
        asm::ControlFlow::Halt.into(),
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
async fn read_pre_post_state_and_check_constraints() {
    let intent_addr = TEST_INTENT_ADDR;

    // In the pre-state, we have [Some(40), None, Some(42)].
    let pre_state = State::new(vec![(
        intent_addr.set.clone(),
        vec![([0, 0, 0, 0], 40), ([0, 0, 0, 2], 42)],
    )]);

    // The full solution that we're checking.
    let solution = Solution {
        data: vec![SolutionData {
            intent_to_solve: intent_addr.clone(),
            decision_variables: vec![],
        }],
        // We have one mutation that sets a missing value to 41.
        state_mutations: vec![StateMutation {
            pathway: 0,
            mutations: vec![Mutation {
                key: [0, 0, 0, 1],
                value: Some(41),
            }],
        }],
        partial_solutions: vec![],
    };

    // The index of the solution data associated with the intent we're solving.
    let intent_index = 0;

    let mutable_keys = mut_keys_set(&solution, intent_index);

    // Construct access to the necessary solution data for the VM.
    let mut access = Access {
        solution: SolutionAccess::new(&solution, intent_index, &mutable_keys),
        // Haven't calculated these yet.
        state_slots: StateSlots::EMPTY,
    };

    // A simple state read program that reads words directly to the slots.
    let ops = &[
        asm::Stack::Push(3).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(0).into(), // Key1
        asm::Stack::Push(0).into(), // Key2
        asm::Stack::Push(0).into(), // Key3
        asm::Stack::Push(3).into(), // Num words
        asm::StateRead::WordRange,
        asm::ControlFlow::Halt.into(),
    ];

    // Execute the program.
    let mut vm = Vm::default();
    vm.exec_ops(ops, access, &pre_state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();

    // Collect the state slots.
    let pre_state_slots = vm.into_state_slots();

    // Apply the state mutations to the state to produce the post state.
    let mut post_state = pre_state.clone();
    for mutation in &solution.state_mutations {
        let solution_data = &solution.data[usize::from(mutation.pathway)];
        let set_addr = &solution_data.intent_to_solve.set;
        for Mutation { key, value } in mutation.mutations.iter() {
            post_state.set(set_addr.clone(), key, *value);
        }
    }

    // Execute the program with the post state.
    let mut vm = Vm::default();
    vm.exec_ops(ops, access, &post_state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();

    // Collect the state slots.
    let post_state_slots = vm.into_state_slots();

    // State slots should have updated.
    assert_eq!(&pre_state_slots[..], &[Some(40), None, Some(42)]);
    assert_eq!(&post_state_slots[..], &[Some(40), Some(41), Some(42)]);

    // Now, they can be used for constraint checking.
    access.state_slots = StateSlots {
        pre: &pre_state_slots[..],
        post: &post_state_slots[..],
    };
    let constraints: &[Vec<u8>] = &[
        // Check that the first pre and post slots are equal.
        constraint::asm::to_bytes(vec![
            asm::Stack::Push(0).into(), // slot
            asm::Stack::Push(0).into(), // pre
            asm::Access::State.into(),
            asm::Stack::Push(0).into(), // slot
            asm::Stack::Push(1).into(), // post
            asm::Access::State.into(),
            asm::Pred::Eq.into(),
        ])
        .collect(),
        // Check that the second pre state is none, but post is some.
        constraint::asm::to_bytes(vec![
            asm::Stack::Push(1).into(), // slot
            asm::Stack::Push(0).into(), // pre
            asm::Access::StateIsSome.into(),
            asm::Pred::Not.into(),
            asm::Stack::Push(1).into(), // slot
            asm::Stack::Push(1).into(), // post
            asm::Access::StateIsSome.into(),
            asm::Pred::And.into(),
        ])
        .collect(),
        // Check that the third pre and post slots are equal.
        constraint::asm::to_bytes(vec![
            asm::Stack::Push(2).into(), // slot
            asm::Stack::Push(0).into(), // pre
            asm::Access::State.into(),
            asm::Stack::Push(2).into(), // slot
            asm::Stack::Push(1).into(), // post
            asm::Access::State.into(),
            asm::Pred::Eq.into(),
        ])
        .collect(),
    ];
    constraint::check_intent(constraints, access).unwrap();

    // Constraints pass - we're free to apply the updated state!
}
