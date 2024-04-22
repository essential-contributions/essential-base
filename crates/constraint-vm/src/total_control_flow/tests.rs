use crate::{asm, exec_ops, test_util::test_access};

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
    let stack = exec_ops(ops, access).unwrap();
    assert_eq!(&stack[..], &[3]);
}
