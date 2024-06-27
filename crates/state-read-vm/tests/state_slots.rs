mod util;

use essential_state_read_vm::{
    asm::{self, Op, Word},
    error::{OpError, OpSyncError, StateReadError, StateSlotsError},
    GasLimit, StateSlotsMut, Vm,
};
use util::*;

#[tokio::test]
async fn alloc() {
    let mut vm = Vm::default();
    let len = 5;
    assert_eq!(vm.state_slots_mut.len(), 0);
    let ops = &[
        asm::Stack::Push(len).into(),
        asm::StateSlots::AllocSlots.into(),
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
    assert_eq!(vm.state_slots_mut.len(), len as usize);
}

#[tokio::test]
async fn len() {
    let mut vm = Vm::default();
    let len = 3;
    assert_eq!(vm.state_slots_mut.len(), 0);
    let ops = &[
        asm::Stack::Push(len).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::StateSlots::Length.into(),
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
    assert_eq!(vm.state_slots_mut.len(), len as usize);
    assert_eq!(&vm.stack[..], &[len]);
}

#[tokio::test]
async fn clear() {
    let mut vm = Vm::default();
    // First, push a value.
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::StateSlots::Store.into(),
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
    assert_eq!(vm.state_slots_mut.len(), 1);
    assert_eq!(&vm.state_slots_mut[..], &[vec![42]]);
    // Next, clear the value.
    let ops = &[
        asm::Stack::Push(0).into(),
        asm::StateSlots::Clear.into(),
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
    assert_eq!(vm.state_slots_mut.len(), 1);
    assert!(&vm.state_slots_mut[0].is_empty());
}

#[tokio::test]
async fn clear_range() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(4).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(21).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(84).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(2).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(168).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(3).into(),
        asm::StateSlots::Store.into(),
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
    assert_eq!(vm.state_slots_mut.len(), 4);
    assert_eq!(
        &vm.state_slots_mut[..],
        &[vec![21], vec![42], vec![84], vec![168]]
    );
    // Next, clear the values at indices 1 and 2.
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(2).into(),
        asm::StateSlots::ClearRange.into(),
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
    // Capacity remains the same, but middle values should be vec![].
    assert_eq!(vm.state_slots_mut.len(), 4);
    assert_eq!(
        &vm.state_slots_mut[..],
        &[vec![21], vec![], vec![], vec![168]]
    );
    assert!(vm.stack.is_empty());
}

#[tokio::test]
async fn length() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(3).into(),
        asm::Stack::Push(0).into(),
        asm::StateSlots::Store.into(),
        asm::StateSlots::Length.into(),
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
    assert_eq!(vm.state_slots_mut.len(), 6);
    assert_eq!(&vm.stack[..], &[6]);
}

#[tokio::test]
async fn load() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(0).into(), // Load the value at index 0
        asm::StateSlots::Load.into(),
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
    assert_eq!(&vm.state_slots_mut[..], &[vec![42]]);
    assert_eq!(&vm.stack[..], &[42]);
}

#[tokio::test]
async fn store() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(2).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(21).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(1).into(),
        asm::StateSlots::Store.into(),
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
    assert_eq!(&vm.state_slots_mut[..], &[vec![], vec![42, 21]]);
}

#[tokio::test]
async fn load_index_oob() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(0).into(),
        asm::StateSlots::Load.into(),
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
            OpError::Sync(OpSyncError::StateSlots(StateSlotsError::IndexOutOfBounds)),
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
        asm::StateSlots::Store.into(),
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
            OpError::Sync(OpSyncError::StateSlots(StateSlotsError::IndexOutOfBounds)),
        )) => (),
        _ => panic!("expected index out of bounds, found {:?}", res),
    }
}

#[tokio::test]
async fn alloc_overflow() {
    let mut vm = Vm::default();
    let overflow_cap = Word::try_from(StateSlotsMut::SLOT_LIMIT.checked_add(1).unwrap()).unwrap();
    let ops = &[
        asm::Stack::Push(overflow_cap).into(),
        asm::StateSlots::AllocSlots.into(),
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
            OpError::Sync(OpSyncError::StateSlots(StateSlotsError::Overflow)),
        )) => (),
        _ => panic!("expected overflow, found {:?}", res),
    }
}

#[tokio::test]
async fn store_word() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(2).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(0).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(3).into(),
        asm::Stack::Push(1).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(42).into(),
        asm::StateSlots::StoreWord.into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(27).into(),
        asm::StateSlots::StoreWord.into(),
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
    assert_eq!(&vm.state_slots_mut[..], &[vec![0, 42], vec![27, 0, 0]]);
}

#[tokio::test]
async fn store_word_oob() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(3).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(0).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(42).into(),
        asm::StateSlots::StoreWord.into(),
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
            OpError::Sync(OpSyncError::StateSlots(StateSlotsError::IndexOutOfBounds)),
        )) => (),
        _ => panic!("expected index oob, found {:?}", res),
    }
}

#[tokio::test]
async fn load_word() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(27).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(0).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(0).into(), // Load the slot at index 0
        asm::Stack::Push(1).into(), // Load the word at index 1
        asm::StateSlots::LoadWord.into(),
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
    assert_eq!(&vm.state_slots_mut[..], &[vec![42, 27]]);
    assert_eq!(&vm.stack[..], &[27]);
}

#[tokio::test]
async fn load_word_oob() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(27).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(0).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(0).into(), // Load the slot at index 0
        asm::Stack::Push(2).into(), // Load the word at index 1
        asm::StateSlots::LoadWord.into(),
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
            OpError::Sync(OpSyncError::StateSlots(StateSlotsError::IndexOutOfBounds)),
        )) => (),
        _ => panic!("expected index oob, found {:?}", res),
    }
}

#[tokio::test]
async fn value_len() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(42).into(),
        asm::Stack::Push(27).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(3).into(),
        asm::Stack::Push(0).into(),
        asm::StateSlots::Store.into(),
        asm::Stack::Push(0).into(), // Get the length of the value at index 0
        asm::StateSlots::ValueLen.into(),
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
    assert_eq!(&vm.state_slots_mut[..], &[vec![42, 27, 1]]);
    assert_eq!(&vm.stack[..], &[3]);
}
