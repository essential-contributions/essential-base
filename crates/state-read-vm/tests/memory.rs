mod util;

use essential_state_read_vm::{
    asm::{self, Op, Word},
    error::{MemoryError, OpError, OpSyncError, StateReadError},
    GasLimit, Memory, Vm,
};
use util::*;

#[tokio::test]
async fn alloc() {
    let mut vm = Vm::default();
    let cap = 5;
    assert_eq!(vm.memory.capacity(), 0);
    let ops = &[
        asm::Stack::Push(cap).into(),
        asm::Memory::Alloc.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(vm.memory.capacity(), cap as usize);
}

#[tokio::test]
async fn capacity() {
    let mut vm = Vm::default();
    let cap = 3;
    assert_eq!(vm.memory.capacity(), 0);
    let ops = &[
        asm::Stack::Push(cap).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::Capacity.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(vm.memory.capacity(), cap as usize);
    assert_eq!(&vm.stack[..], &[cap]);
}

#[tokio::test]
async fn clear() {
    let mut vm = Vm::default();
    // First, push a value.
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(vm.memory.capacity(), 1);
    assert_eq!(&vm.memory[..], &[Some(42)]);
    // Next, clear the value.
    let ops = &[
        asm::Stack::Push(0).into(),
        asm::Memory::Clear.into(),
        asm::ControlFlow::Halt.into(),
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
    // Capacity remains the same. But the value is `None`.
    assert_eq!(vm.memory.capacity(), 1);
    assert_eq!(&vm.memory[..], &[None]);
}

#[tokio::test]
async fn clear_range() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(4).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(vm.memory.capacity(), 4);
    assert_eq!(&vm.memory[..], &[Some(42), Some(42), Some(42), Some(42)]);
    // Next, clear the values at indices 1 and 2.
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(2).into(),
        asm::Memory::ClearRange.into(),
        asm::ControlFlow::Halt.into(),
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
    // Capacity remains the same, but middle values should be None.
    assert_eq!(vm.memory.capacity(), 4);
    assert_eq!(&vm.memory[..], &[Some(42), None, None, Some(42)]);
    assert!(vm.stack.is_empty());
}

#[tokio::test]
async fn free() {
    let mut vm = Vm::default();
    let size = 3;
    assert_eq!(vm.memory.capacity(), 0);
    let ops = &[
        asm::Stack::Push(size).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(size).into(),
        asm::Memory::Free.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(vm.memory.capacity(), 0);
}

#[tokio::test]
async fn is_some() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::Stack::Push(0).into(), // Check if the value at index 0 is `Some`
        asm::Memory::IsSome.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(&vm.stack[..], &[1 /*true*/],);
}

#[tokio::test]
async fn length() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::Memory::Length.into(),
        asm::ControlFlow::Halt.into(),
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
    // Make sure that capacity and length are tracked separately correctly.
    assert_eq!(vm.memory.capacity(), 6);
    assert_eq!(vm.memory.len(), 3);
    assert_eq!(&vm.stack[..], &[3]);
}

#[tokio::test]
async fn load() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::Stack::Push(0).into(), // Load the value at index 0
        asm::Memory::Load.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(&vm.memory[..], &[Some(42)]);
    assert_eq!(&vm.stack[..], &[42]);
}

#[tokio::test]
async fn push() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(vm.memory.capacity(), 1);
    assert_eq!(&vm.memory[..], &[Some(42)]);
    assert!(vm.stack.is_empty());
}

#[tokio::test]
async fn push_none() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(2).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::PushNone.into(),
        asm::Memory::PushNone.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(vm.memory.capacity(), 2);
    assert_eq!(&vm.memory[..], &[None, None]);
    assert!(vm.stack.is_empty());
}

#[tokio::test]
async fn store() {
    let mut vm = Vm::default();
    let ops = &[
        // Allocate two slots.
        asm::Stack::Push(2).into(),
        asm::Memory::Alloc.into(),
        // Push two `None`s onto the allocated memory.
        asm::Memory::PushNone.into(),
        asm::Memory::PushNone.into(),
        // Store `Some(42)` in the second slot.
        asm::Stack::Push(1).into(),
        asm::Stack::Push(42).into(),
        asm::Memory::Store.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(&vm.memory[..], &[None, Some(42)]);
}

#[tokio::test]
async fn truncate() {
    let mut vm = Vm::default();
    let ops = &[
        // Push 3 `None`s.
        asm::Stack::Push(3).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::PushNone.into(),
        asm::Memory::PushNone.into(),
        asm::Memory::PushNone.into(),
        // Truncate down to one `None`. Doesn't affect capacity.
        asm::Stack::Push(1).into(),
        asm::Memory::Truncate.into(),
        asm::ControlFlow::Halt.into(),
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
    assert_eq!(&vm.memory[..], &[None]);
    assert_eq!(vm.memory.capacity(), 3);
}

#[tokio::test]
async fn load_index_oob() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::ControlFlow::Halt.into(),
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
            OpError::Sync(OpSyncError::Memory(MemoryError::IndexOutOfBounds)),
        )) => (),
        _ => panic!("expected index out of bounds, found {:?}", res),
    }
}

#[tokio::test]
async fn store_index_oob() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
        asm::ControlFlow::Halt.into(),
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
            OpError::Sync(OpSyncError::Memory(MemoryError::IndexOutOfBounds)),
        )) => (),
        _ => panic!("expected index out of bounds, found {:?}", res),
    }
}

#[tokio::test]
async fn push_overflow() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(42).into(),
        asm::Memory::Push.into(),
        asm::ControlFlow::Halt.into(),
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
        Err(StateReadError::Op(_, OpError::Sync(OpSyncError::Memory(MemoryError::Overflow)))) => (),
        _ => panic!("expected overflow, found {:?}", res),
    }
}

#[tokio::test]
async fn alloc_overflow() {
    let mut vm = Vm::default();
    let overflow_cap = Word::try_from(Memory::SIZE_LIMIT.checked_add(1).unwrap()).unwrap();
    let ops = &[
        asm::Stack::Push(overflow_cap).into(),
        asm::Memory::Alloc.into(),
        asm::ControlFlow::Halt.into(),
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
        Err(StateReadError::Op(_, OpError::Sync(OpSyncError::Memory(MemoryError::Overflow)))) => (),
        _ => panic!("expected overflow, found {:?}", res),
    }
}
