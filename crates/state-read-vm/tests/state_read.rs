mod util;

use essential_state_read_vm::{
    asm::{self, Op},
    types::{convert::word_4_from_u8_32, ContentAddress},
    GasLimit, Vm,
};
use util::*;

#[tokio::test]
async fn state_read_3_42s() {
    let access = TEST_ACCESS;
    let state = State::new(vec![(
        access.solution.this_data().intent_to_solve.set.clone(),
        vec![([0, 0, 0, 0], 42), ([0, 0, 0, 1], 42), ([0, 0, 0, 2], 42)],
    )]);
    let mut vm = Vm::default();
    let num_words = 3;
    let ops = &[
        asm::Stack::Push(num_words).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(0).into(), // Key1
        asm::Stack::Push(0).into(), // Key2
        asm::Stack::Push(0).into(), // Key3
        asm::Stack::Push(num_words).into(),
        asm::StateRead::WordRange,
        asm::ControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(&vm.memory[..], &[Some(42), Some(42), Some(42)]);
    assert_eq!(vm.memory.capacity(), 3);
}

#[tokio::test]
async fn state_read_some_none_some() {
    let access = TEST_ACCESS;
    let state = State::new(vec![(
        access.solution.this_data().intent_to_solve.set.clone(),
        vec![([0, 0, 0, 0], 42), ([0, 0, 0, 2], 42)],
    )]);
    let mut vm = Vm::default();
    let num_words = 3;
    let ops = &[
        asm::Stack::Push(num_words).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(0).into(), // Key1
        asm::Stack::Push(0).into(), // Key2
        asm::Stack::Push(0).into(), // Key3
        asm::Stack::Push(num_words).into(),
        asm::StateRead::WordRange,
        asm::ControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(&vm.memory[..], &[Some(42), None, Some(42)]);
    assert_eq!(vm.memory.capacity(), 3);
}

#[tokio::test]
async fn state_read_ext() {
    let ext_set_addr = ContentAddress([0x12; 32]);
    let state = State::new(vec![(
        ext_set_addr.clone(),
        vec![([1, 2, 3, 4], 40), ([1, 2, 3, 5], 41), ([1, 2, 3, 6], 42)],
    )]);
    let mut vm = Vm::default();
    let num_words = 3;
    let [addr0, addr1, addr2, addr3] = word_4_from_u8_32(ext_set_addr.0);
    let ops = &[
        asm::Stack::Push(num_words).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(addr0).into(),
        asm::Stack::Push(addr1).into(),
        asm::Stack::Push(addr2).into(),
        asm::Stack::Push(addr3).into(),
        asm::Stack::Push(1).into(), // Key0
        asm::Stack::Push(2).into(), // Key1
        asm::Stack::Push(3).into(), // Key2
        asm::Stack::Push(4).into(), // Key3
        asm::Stack::Push(num_words).into(),
        asm::StateRead::WordRangeExtern,
        asm::ControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, TEST_ACCESS, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(&vm.memory[..], &[Some(40), Some(41), Some(42)]);
    assert_eq!(vm.memory.capacity(), 3);
}

#[tokio::test]
async fn state_read_ext_nones() {
    let ext_set_addr = ContentAddress([0x12; 32]);
    let state = State::new(vec![(ext_set_addr.clone(), vec![])]);
    let mut vm = Vm::default();
    let num_words = 3;
    let [addr0, addr1, addr2, addr3] = word_4_from_u8_32(ext_set_addr.0);
    let ops = &[
        asm::Stack::Push(num_words).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(addr0).into(),
        asm::Stack::Push(addr1).into(),
        asm::Stack::Push(addr2).into(),
        asm::Stack::Push(addr3).into(),
        asm::Stack::Push(1).into(), // Key0
        asm::Stack::Push(2).into(), // Key1
        asm::Stack::Push(3).into(), // Key2
        asm::Stack::Push(4).into(), // Key3
        asm::Stack::Push(num_words).into(),
        asm::StateRead::WordRangeExtern,
        asm::ControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, TEST_ACCESS, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(&vm.memory[..], &[None, None, None]);
    assert_eq!(vm.memory.capacity(), 3);
}
