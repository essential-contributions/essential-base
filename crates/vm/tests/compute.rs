mod util;

use essential_asm::Word;
use essential_vm::{
    asm::{self, Op},
    Gas, GasLimit, Vm,
};
use util::*;

// Post-compute memory is a concatenation of compute indices.
#[test]
fn test_compute() {
    let mut vm = Vm::default();
    let compute_breadth = 1000;
    let ops = &[
        asm::Stack::Push(compute_breadth).into(), // compute in 1000 threads
        asm::Compute::Compute.into(),
        asm::Stack::Push(1).into(), // alloc 1 word in memory
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(),       // store compute index in memory
        asm::Compute::ComputeEnd.into(), // end compute
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

    assert_eq!(vm.pc, ops.len());
    // parent memory is a concatenation of children's memories
    assert_eq!(&vm.memory[..], (0..compute_breadth).collect::<Vec<Word>>());
    assert!(&vm.stack.is_empty());
    // total gas spent
    //     = cost of ops before compute
    //         + (#compute * cost of ops in compute)
    assert_eq!(
        spent,
        ops[..2].iter().map(op_gas_cost).sum::<Gas>()
            + (compute_breadth as u64 * (ops[2..].iter().map(op_gas_cost).sum::<Gas>()))
    );
}

// Parent VM stack functions as expected.
#[test]
fn test_compute_stack() {
    let mut vm = Vm::default();
    let compute_breadth = 1000;
    let ops = &[
        asm::Stack::Push(41).into(),
        asm::Stack::Push(compute_breadth).into(),
        asm::Compute::Compute.into(),
        asm::Stack::Pop.into(), // stack operation in compute
        asm::Compute::ComputeEnd.into(),
        asm::Stack::Pop.into(),
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

    assert_eq!(vm.pc, ops.len());
    assert!(&vm.memory.is_empty());
    // stack operation in compute not reflected in parent stack
    assert_eq!(&vm.stack[..], &[42]);
    // total gas spent
    //     = cost of ops before compute
    //         + (#compute * cost of ops in compute)
    //             + cost of ops after compute
    assert_eq!(
        spent,
        ops[..3].iter().map(op_gas_cost).sum::<Gas>()
            + (compute_breadth as u64 * (ops[3..5].iter().map(op_gas_cost).sum::<Gas>())
                + ops[5..].iter().map(op_gas_cost).sum::<Gas>())
    );
}

// Test that compute end is not required to end the VM after compute.
// Behaves identically to [`test_compute`].
#[test]
fn test_compute_end() {
    let mut vm = Vm::default();
    let compute_breadth = 1000;
    let ops = &[
        asm::Stack::Push(compute_breadth).into(), // compute in 1000 threads
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

    assert_eq!(vm.pc, ops.len());
    // parent memory is a concatenation of children's memories
    assert_eq!(&vm.memory[..], (0..compute_breadth).collect::<Vec<Word>>());
    assert!(&vm.stack.is_empty());
    // total gas spent
    //     = cost of ops before compute
    //         + (#compute * cost of ops in compute)
    assert_eq!(
        spent,
        ops[..2].iter().map(op_gas_cost).sum::<Gas>()
            + (compute_breadth as u64 * (ops[2..].iter().map(op_gas_cost).sum::<Gas>()))
    );
}
