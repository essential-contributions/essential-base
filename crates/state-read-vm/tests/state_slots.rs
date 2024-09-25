mod util;

use essential_state_read_vm::{
    asm::{self, Op, Word},
    error::{OpError, OpSyncError, StateMemoryError, StateReadError},
    GasLimit, StateMemory, Vm,
};
use util::*;

#[tokio::test]
async fn alloc() {
    let mut vm = Vm::default();
    let len = 5;
    assert_eq!(vm.state_memory.len(), 0);
    let ops = &[
        asm::Stack::Push(len).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(vm.state_memory.len(), len as usize);
}

#[tokio::test]
async fn len() {
    let mut vm = Vm::default();
    let len = 3;
    assert_eq!(vm.state_memory.len(), 0);
    let ops = &[
        asm::Stack::Push(len).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::StateMemory::Length.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(vm.state_memory.len(), len as usize);
    assert_eq!(&vm.stack[..], &[len]);
}

#[tokio::test]
async fn truncate() {
    let mut vm = Vm::default();
    // First, push a value.
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(1).into(),
        asm::StateMemory::Store.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(vm.state_memory.len(), 1);
    assert_eq!(&vm.state_memory[..], &[vec![42]]);
    // Next, clear the value.
    let ops = &[
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::StateMemory::Truncate.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.pc = 0;
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    // Capacity remains the same. But the value is `vec![]`.
    assert_eq!(vm.state_memory.len(), 1);
    assert!(&vm.state_memory[0].is_empty());
}

#[tokio::test]
async fn length() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(3).into(),
        asm::StateMemory::Store.into(),
        asm::StateMemory::Length.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(vm.state_memory.len(), 6);
    assert_eq!(&vm.stack[..], &[6]);
}

#[tokio::test]
async fn load() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(1).into(),
        asm::StateMemory::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::StateMemory::Load.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(&vm.state_memory[..], &[vec![42]]);
    assert_eq!(&vm.stack[..], &[42]);
}

#[tokio::test]
async fn store() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(2).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(21).into(),
        asm::Stack::Push(2).into(),
        asm::StateMemory::Store.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(&vm.state_memory[..], &[vec![], vec![42, 21]]);
}

#[tokio::test]
async fn load_index_oob() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::StateMemory::Load.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    let res = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await;
    match res {
        Err(StateReadError::Op(
            _,
            OpError::Sync(OpSyncError::StateSlots(StateMemoryError::IndexOutOfBounds)),
        )) => (),
        _ => panic!("expected index out of bounds, found {:?}", res),
    }
}

#[tokio::test]
async fn store_index_oob() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::StateMemory::Store.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    let res = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await;
    match res {
        Err(StateReadError::Op(
            _,
            OpError::Sync(OpSyncError::StateSlots(StateMemoryError::IndexOutOfBounds)),
        )) => (),
        _ => panic!("expected index out of bounds, found {:?}", res),
    }
}

#[tokio::test]
async fn alloc_overflow() {
    let mut vm = Vm::default();
    let overflow_cap = Word::try_from(StateMemory::SLOT_LIMIT.checked_add(1).unwrap()).unwrap();
    let ops = &[
        asm::Stack::Push(overflow_cap).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    let res = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await;
    match res {
        Err(StateReadError::Op(
            _,
            OpError::Sync(OpSyncError::StateSlots(StateMemoryError::Overflow)),
        )) => (),
        _ => panic!("expected overflow, found {:?}", res),
    }
}

#[tokio::test]
async fn store_word() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(2).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(2).into(),
        asm::StateMemory::Store.into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(3).into(),
        asm::StateMemory::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(1).into(),
        asm::StateMemory::Store.into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(27).into(),
        asm::Stack::Push(1).into(),
        asm::StateMemory::Store.into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(72).into(),
        asm::Stack::Push(72).into(),
        asm::Stack::Push(2).into(),
        asm::StateMemory::Store.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(&vm.state_memory[..], &[vec![0, 42], vec![27, 0, 72, 72]]);
}

#[tokio::test]
async fn store_word_oob() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(3).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(2).into(),
        asm::StateMemory::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(3).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(1).into(),
        asm::StateMemory::Store.into(),
    ];
    let res = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await;
    match res {
        Err(StateReadError::Op(
            _,
            OpError::Sync(OpSyncError::StateSlots(StateMemoryError::IndexOutOfBounds)),
        )) => (),
        _ => panic!("expected index oob, found {:?}", res),
    }
}

#[tokio::test]
async fn load_word() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(27).into(),
        asm::Stack::Push(2).into(),
        asm::StateMemory::Store.into(),
        asm::Stack::Push(0).into(), // slot index
        asm::Stack::Push(1).into(), // value index
        asm::Stack::Push(1).into(), // length
        asm::StateMemory::Load.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(&vm.state_memory[..], &[vec![42, 27]]);
    assert_eq!(&vm.stack[..], &[27]);
}

#[tokio::test]
async fn load_word_oob() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(27).into(),
        asm::Stack::Push(2).into(),
        asm::StateMemory::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(1).into(),
        asm::StateMemory::Load.into(),
    ];
    let res = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await;
    match res {
        Err(StateReadError::Op(
            _,
            OpError::Sync(OpSyncError::StateSlots(StateMemoryError::IndexOutOfBounds)),
        )) => (),
        _ => panic!("expected index oob, found {:?}", res),
    }
}

#[tokio::test]
async fn value_len() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateMemory::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(27).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(3).into(),
        asm::StateMemory::Store.into(),
        asm::Stack::Push(0).into(), // Get the length of the value at index 0
        asm::StateMemory::ValueLen.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &State::EMPTY,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(&vm.state_memory[..], &[vec![42, 27, 1]]);
    assert_eq!(&vm.stack[..], &[3]);
}
