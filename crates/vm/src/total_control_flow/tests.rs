use crate::{
    asm,
    error::{OpError, TotalControlFlowError},
    sync::test_util::test_access,
    utils::EmptyState,
    GasLimit, Op, Vm,
};

#[test]
fn test_jump_if() {
    let access = test_access();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(3).into(),
        asm::Stack::Push(1).into(),
        asm::TotalControlFlow::JumpIf.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let mut vm = Vm::default();
    vm.exec_ops(
        ops,
        access.clone(),
        &EmptyState,
        op_gas_cost,
        GasLimit::UNLIMITED,
    )
    .unwrap();
    assert_eq!(&vm.stack[..], &[3]);

    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(3).into(),
        asm::Stack::Push(0).into(),
        asm::TotalControlFlow::JumpIf.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let mut vm = Vm::default();
    vm.exec_ops(
        ops,
        access.clone(),
        &EmptyState,
        op_gas_cost,
        GasLimit::UNLIMITED,
    )
    .unwrap();
    assert_eq!(&vm.stack[..], &[4]);
}

#[test]
fn test_halt_if() {
    let access = test_access();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(1).into(),
        asm::TotalControlFlow::HaltIf.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let mut vm = Vm::default();
    vm.exec_ops(
        ops,
        access.clone(),
        &EmptyState,
        op_gas_cost,
        GasLimit::UNLIMITED,
    )
    .unwrap();
    assert_eq!(&vm.stack[..], &[2]);

    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::TotalControlFlow::HaltIf.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let mut vm = Vm::default();
    vm.exec_ops(
        ops,
        access.clone(),
        &EmptyState,
        op_gas_cost,
        GasLimit::UNLIMITED,
    )
    .unwrap();
    assert_eq!(&vm.stack[..], &[3]);
}

#[test]
fn test_panic_if() {
    let mut stack = crate::Stack::default();
    stack.push(42).unwrap();
    stack.push(43).unwrap();
    stack.push(1).unwrap();

    let err = super::panic_if(&mut stack).unwrap_err();
    assert!(err.to_string().ends_with("[42, 43]"),);
    assert!(
        matches!(err, OpError::TotalControlFlow(TotalControlFlowError::Panic(s)) if s == vec![42, 43])
    );

    let mut stack = crate::Stack::default();
    stack.push(1).unwrap();

    let err = super::panic_if(&mut stack).unwrap_err();
    assert!(err.to_string().ends_with("[]"),);
    assert!(
        matches!(err, OpError::TotalControlFlow(TotalControlFlowError::Panic(s)) if s.is_empty())
    );

    let mut stack = crate::Stack::default();
    stack.push(42).unwrap();
    stack.push(43).unwrap();
    stack.push(0).unwrap();

    super::panic_if(&mut stack).unwrap();
    assert_eq!(stack.len(), 2);
    assert_eq!(stack.pop().unwrap(), 43);
    assert_eq!(stack.pop().unwrap(), 42);

    let mut stack = crate::Stack::default();
    stack.push(42).unwrap();
    stack.push(43).unwrap();
    let err = super::panic_if(&mut stack).unwrap_err();
    assert!(matches!(
        err,
        OpError::TotalControlFlow(TotalControlFlowError::InvalidPanicIfCondition)
    ));
}
