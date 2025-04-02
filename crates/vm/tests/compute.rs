mod util;

use essential_asm::{Compute, Word};
use essential_vm::{
    asm::{self, Op},
    Gas, GasLimit, Vm,
};
use util::*;

// Gas cost of ops starting from first op until `Compute` op, inclusive.
type PreComputeGas = Gas;
// Gas cost of ops starting after `Compute` op until `ComputeEnd`op, inclusive.
type ComputeGas = Gas;
// Gas cost of ops starting after `ComputeEnd` op until the last op.
type PostComputeGas = Gas;

// Helper to calculate total gas spent.
// total_gas_spent
//   = pre_compute_gas
//     + (compute_breadth * compute_gas)
//       + post_compute_gas
fn compute_ops(ops: &[Op]) -> (PreComputeGas, ComputeGas, PostComputeGas) {
    let op_gas_cost = &|_: &Op| 1;
    let compute_index = ops
        .iter()
        .position(|&op| op == Op::Compute(Compute::Compute))
        .unwrap();
    let compute_end_index = ops
        .iter()
        .position(|&op| op == Op::Compute(Compute::ComputeEnd))
        .unwrap();
    let pre_compute_gas = ops[..=compute_index].iter().map(op_gas_cost).sum::<Gas>();
    let compute_gas = ops[compute_index + 1..=compute_end_index]
        .iter()
        .map(op_gas_cost)
        .sum::<Gas>();
    let post_compute_gas = ops[compute_end_index + 1..]
        .iter()
        .map(op_gas_cost)
        .sum::<Gas>();

    (pre_compute_gas, compute_gas, post_compute_gas)
}

// Post-compute memory functions as expected.
#[test]
fn test_compute_memory() {
    let mut vm = Vm::default();
    let compute_breadth = 1000;
    let ops = &[
        // store 41 in memory
        asm::Stack::Push(41).into(),
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(),
        // compute in 1000 programs
        asm::Stack::Push(compute_breadth).into(),
        asm::Compute::Compute.into(),
        asm::Stack::Push(1).into(), // alloc 1 word in memory
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(), // store compute index in memory
        // end compute
        asm::Compute::ComputeEnd.into(),
        // store 42 in memory
        asm::Stack::Push(42).into(),
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(),
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

    // calculate expected gas
    let (pre_compute_gas, compute_gas, post_compute_gas) = compute_ops(ops);
    let expected_spent = pre_compute_gas + compute_breadth as u64 * compute_gas + post_compute_gas;

    assert_eq!(vm.pc, ops.len());
    // parent memory is [41, ..concatenation of children's memories, 42]
    assert_eq!(
        &vm.memory[..],
        vec![41]
            .into_iter()
            .chain((0..compute_breadth).map(|i| i))
            .chain(std::iter::once(42))
            .collect::<Vec<Word>>()
    );
    assert!(&vm.stack.is_empty());
    assert_eq!(spent, expected_spent);
}

// Parent VM stack functions as expected.
#[test]
fn test_compute_stack() {
    let mut vm = Vm::default();
    let compute_breadth = 1000;
    let ops = &[
        // push 41 to stack
        asm::Stack::Push(41).into(),
        // compute in 1000 programs
        asm::Stack::Push(compute_breadth).into(),
        asm::Compute::Compute.into(),
        asm::Stack::Pop.into(), // pop stack in compute
        // push 40 to stack in compute
        asm::Stack::Push(40).into(),
        // end compute
        asm::Compute::ComputeEnd.into(),
        // push 42 to stack
        asm::Stack::Push(42).into(),
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

    // calculate expected gas
    let (pre_compute_gas, compute_gas, post_compute_gas) = compute_ops(ops);
    let expected_spent = pre_compute_gas + compute_breadth as u64 * compute_gas + post_compute_gas;

    assert_eq!(vm.pc, ops.len());
    assert!(&vm.memory.is_empty());
    // stack operation in compute not reflected in parent stack
    assert_eq!(&vm.stack[..], &[41, 42]);
    assert_eq!(spent, expected_spent);
}

// Test that compute end is not required to end the VM after compute.
// Behaves identically to [`test_compute`].
#[test]
fn test_compute_end() {
    let mut vm = Vm::default();
    let compute_breadth = 1000;
    let ops = &[
        // compute in 1000 threads
        asm::Stack::Push(compute_breadth).into(),
        asm::Compute::Compute.into(),
        asm::Stack::Push(1).into(), // alloc 1 word in memory
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(), // store compute index in memory
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

    // calculate expected gas
    let pre_compute_gas = ops[..2].iter().map(op_gas_cost).sum::<Gas>();
    let compute_gas = ops[2..].iter().map(op_gas_cost).sum::<Gas>();
    let expected_spent = pre_compute_gas + compute_breadth as u64 * compute_gas;

    assert_eq!(vm.pc, ops.len());
    // parent memory is a concatenation of children's memories
    assert_eq!(&vm.memory[..], (0..compute_breadth).collect::<Vec<Word>>());
    assert!(&vm.stack.is_empty());
    assert_eq!(spent, expected_spent);
}

// Test that halt in compute program exits the entire program.
#[ignore = "todo"]
#[test]
fn test_compute_halt() {
    let mut vm = Vm::default();
    let compute_breadth = 1000;
    let ops = &[
        // compute in 1000 threads
        asm::Stack::Push(compute_breadth).into(),
        asm::Compute::Compute.into(),
        asm::TotalControlFlow::Halt.into(),
        asm::Compute::ComputeEnd.into(),
        asm::Stack::Push(42).into(),
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

    // calculate expected gas
    let pre_compute_gas = ops[..2].iter().map(op_gas_cost).sum::<Gas>();
    let compute_gas = ops[2..4].iter().map(op_gas_cost).sum::<Gas>();
    let expected_spent = pre_compute_gas + compute_breadth as u64 * compute_gas;

    // last two ops are not executed due to Halt
    assert_eq!(vm.pc, ops.len() - 2);
    assert!(&vm.memory.is_empty());
    // push to stack in parent after child sees Halt is not executed
    assert!(&vm.stack.is_empty());
    assert_eq!(spent, expected_spent);
}
