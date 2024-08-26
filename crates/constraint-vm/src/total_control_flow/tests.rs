use essential_constraint_asm::Op;

use crate::{asm, exec_ops, test_util::test_access, Gas};

#[test]
fn test_jump_if() {
    let access = *test_access();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(3).into(),
        asm::Stack::Push(1).into(),
        asm::TotalControlFlow::JumpForwardIf.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[3]);

    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(3).into(),
        asm::Stack::Push(0).into(),
        asm::TotalControlFlow::JumpForwardIf.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[4]);
}

#[test]
fn test_halt_if() {
    let access = *test_access();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(1).into(),
        asm::TotalControlFlow::HaltIf.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[2]);

    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::TotalControlFlow::HaltIf.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[3]);
}
