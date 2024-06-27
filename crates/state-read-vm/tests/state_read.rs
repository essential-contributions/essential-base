mod util;

use essential_state_read_vm::{
    asm::{self, Op},
    types::{convert::word_4_from_u8_32, ContentAddress},
    GasLimit, Vm,
};
use util::*;

#[tokio::test]
async fn state_read_3_42s() {
    let access = *test_access();
    let state = State::new(vec![(
        access
            .solution
            .this_data()
            .predicate_to_solve
            .contract
            .clone(),
        vec![
            (vec![0, 0, 0, 0], vec![42]),
            (vec![0, 0, 0, 1], vec![42]),
            (vec![0, 0, 0, 2], vec![42]),
        ],
    )]);
    let mut vm = Vm::default();
    let num_words = 3;
    let ops = &[
        asm::Stack::Push(num_words).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(0).into(), // Key1
        asm::Stack::Push(0).into(), // Key2
        asm::Stack::Push(0).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_words).into(),
        asm::Stack::Push(0).into(), // slot index
        asm::StateRead::KeyRange,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(&vm.state_slots_mut[..], &[vec![42], vec![42], vec![42]]);
    assert_eq!(vm.state_slots_mut.len(), 3);
}

#[tokio::test]
async fn state_read_some_none_some() {
    let access = *test_access();
    let state = State::new(vec![(
        access
            .solution
            .this_data()
            .predicate_to_solve
            .contract
            .clone(),
        vec![(vec![0, 0, 0, 0], vec![42]), (vec![0, 0, 0, 2], vec![42])],
    )]);
    let mut vm = Vm::default();
    let num_words = 3;
    let ops = &[
        asm::Stack::Push(num_words).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(0).into(), // Key1
        asm::Stack::Push(0).into(), // Key2
        asm::Stack::Push(0).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_words).into(),
        asm::Stack::Push(0).into(), // slot index
        asm::StateRead::KeyRange,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(&vm.state_slots_mut[..], &[vec![42], vec![], vec![42]]);
    assert_eq!(vm.state_slots_mut.len(), 3);
}

#[tokio::test]
async fn state_read_ext() {
    let ext_contract_addr = ContentAddress([0x12; 32]);
    let state = State::new(vec![(
        ext_contract_addr.clone(),
        vec![
            (vec![1, 2, 3, 4], vec![40]),
            (vec![1, 2, 3, 5], vec![41]),
            (vec![1, 2, 3, 6], vec![42]),
        ],
    )]);
    let mut vm = Vm::default();
    let num_words = 3;
    let [addr0, addr1, addr2, addr3] = word_4_from_u8_32(ext_contract_addr.0);
    let ops = &[
        asm::Stack::Push(num_words).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(addr0).into(),
        asm::Stack::Push(addr1).into(),
        asm::Stack::Push(addr2).into(),
        asm::Stack::Push(addr3).into(),
        asm::Stack::Push(1).into(), // Key0
        asm::Stack::Push(2).into(), // Key1
        asm::Stack::Push(3).into(), // Key2
        asm::Stack::Push(4).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_words).into(),
        asm::Stack::Push(0).into(), // slot index
        asm::StateRead::KeyRangeExtern,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &state,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert_eq!(&vm.state_slots_mut[..], &[vec![40], vec![41], vec![42]]);
    assert_eq!(vm.state_slots_mut.len(), 3);
}

#[tokio::test]
async fn state_read_ext_nones() {
    let ext_contract_addr = ContentAddress([0x12; 32]);
    let state = State::new(vec![(ext_contract_addr.clone(), vec![])]);
    let mut vm = Vm::default();
    let num_words = 3;
    let [addr0, addr1, addr2, addr3] = word_4_from_u8_32(ext_contract_addr.0);
    let ops = &[
        asm::Stack::Push(num_words).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(addr0).into(),
        asm::Stack::Push(addr1).into(),
        asm::Stack::Push(addr2).into(),
        asm::Stack::Push(addr3).into(),
        asm::Stack::Push(1).into(), // Key0
        asm::Stack::Push(2).into(), // Key1
        asm::Stack::Push(3).into(), // Key2
        asm::Stack::Push(4).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_words).into(),
        asm::Stack::Push(0).into(), // slot index
        asm::StateRead::KeyRangeExtern,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(
        ops,
        *test_access(),
        &state,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .unwrap();
    assert!(vm.state_slots_mut.iter().all(Vec::is_empty));
    assert_eq!(vm.state_slots_mut.len(), 3);
}

#[tokio::test]
async fn state_read_various_size_values() {
    let access = *test_access();
    let state = State::new(vec![(
        access
            .solution
            .this_data()
            .predicate_to_solve
            .contract
            .clone(),
        vec![
            (vec![0, 0, 0, 0], vec![0; 2]),
            (vec![0, 0, 0, 1], vec![1; 22]),
            (vec![0, 0, 0, 2], vec![2; 14]),
            (vec![0, 0, 0, 4], vec![4; 12]),
        ],
    )]);
    let mut vm = Vm::default();
    let num_values = 5;
    let ops = &[
        asm::Stack::Push(num_values).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(0).into(), // Key1
        asm::Stack::Push(0).into(), // Key2
        asm::Stack::Push(0).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_values).into(),
        asm::Stack::Push(0).into(), // slot index
        asm::StateRead::KeyRange,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(
        &vm.state_slots_mut[..],
        &[vec![0; 2], vec![1; 22], vec![2; 14], vec![], vec![4; 12]]
    );
    assert_eq!(vm.state_slots_mut.len(), 5);
}

#[tokio::test]
async fn state_read_various_key_sizes() {
    let access = *test_access();
    let state = State::new(vec![(
        access
            .solution
            .this_data()
            .predicate_to_solve
            .contract
            .clone(),
        vec![
            (vec![0], vec![0; 2]),
            (vec![0, 1], vec![7; 6]),
            (vec![1], vec![1; 22]),
            (vec![0, 0, 0, 0, 0, 2], vec![2; 14]),
            (vec![4; 1000], vec![4; 12]),
        ],
    )]);
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(5).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(1).into(), // key length
        asm::Stack::Push(3).into(), // num keys
        asm::Stack::Push(0).into(), // slot index
        asm::StateRead::KeyRange,
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(6).into(), // key length
        asm::Stack::Push(2).into(), // num keys
        asm::Stack::Push(3).into(), // slot index
        asm::StateRead::KeyRange,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(
        &vm.state_slots_mut[..],
        &[vec![0; 2], vec![1; 22], vec![], vec![2; 14], vec![]]
    );
    assert_eq!(vm.state_slots_mut.len(), 5);
}

// TODO: Test slot index overflow
#[tokio::test]
async fn state_read_slot_index_overflow() {
    let access = *test_access();
    let state = State::new(vec![(
        access
            .solution
            .this_data()
            .predicate_to_solve
            .contract
            .clone(),
        vec![(vec![0], vec![0; 2])],
    )]);
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::StateSlots::AllocSlots.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(1).into(), // key length
        asm::Stack::Push(1).into(), // num keys
        asm::Stack::Push(1).into(), // slot index
        asm::StateRead::KeyRange,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap_err();
}
