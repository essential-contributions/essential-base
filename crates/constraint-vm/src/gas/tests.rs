use crate::error::{ConstraintError, OpError};
use crate::test_util::test_access;
use crate::{asm, exec_ops};

use super::*;

#[test]
fn test_gas() {
    let access = *test_access();
    let ops: Vec<asm::Op> = (0..100)
        .map(|i| {
            if i % 2 == 0 {
                asm::Stack::Push(1)
            } else {
                asm::Stack::Pop
            }
        })
        .map(asm::Op::from)
        .collect();

    exec_ops(&ops, access, &|_: &asm::Op| 1, Gas::MAX).unwrap();

    exec_ops(&ops, access, &|_: &asm::Op| 1, 100).unwrap();

    let e = exec_ops(&ops, access, &|_: &asm::Op| 1, 99).unwrap_err();
    assert!(matches!(e, ConstraintError::Op(_, OpError::OutOfGas(_))));

    exec_ops(&ops, access, &|_: &asm::Op| 0, 1).unwrap();

    exec_ops(
        &ops,
        access,
        &|op: &asm::Op| match op {
            asm::Op::Stack(asm::Stack::Pop) => 2,
            _ => 1,
        },
        150,
    )
    .unwrap();

    let e = exec_ops(
        &ops,
        access,
        &|op: &asm::Op| match op {
            asm::Op::Stack(asm::Stack::Pop) => 2,
            _ => 1,
        },
        149,
    )
    .unwrap_err();
    assert!(matches!(e, ConstraintError::Op(_, OpError::OutOfGas(_))));
}
