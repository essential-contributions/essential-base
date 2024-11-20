use essential_types::Word;
use essential_vm::{
    asm::{self, Op},
    types::{convert::word_4_from_u8_32, ContentAddress},
    GasLimit, Vm,
};
use util::*;

mod util;

#[tokio::test]
async fn state_read_3_42s() {
    let access = *test_access();
    let state = State::new(vec![(
        access.this_data().predicate_to_solve.contract.clone(),
        vec![
            (vec![0, 0, 0, 0], vec![42]),
            (vec![0, 0, 0, 1], vec![42]),
            (vec![0, 0, 0, 2], vec![42]),
        ],
    )]);
    let mut vm = Vm::default();
    let num_keys = 3;
    let num_words = 3;
    let mem_len = num_keys * 2 + num_words;
    let ops = &[
        asm::Stack::Push(mem_len).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(0).into(), // Key1
        asm::Stack::Push(0).into(), // Key2
        asm::Stack::Push(0).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_keys).into(),
        asm::Stack::Push(0).into(), // mem addr
        asm::StateRead::KeyRange,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(vm.memory[..].len(), mem_len as usize);
    assert_eq!(&vm.memory[..], &[6, 1, 7, 1, 8, 1, 42, 42, 42]);
}

#[tokio::test]
async fn state_read_some_none_some() {
    let access = *test_access();
    let state = State::new(vec![(
        access.this_data().predicate_to_solve.contract.clone(),
        vec![(vec![0, 0, 0, 0], vec![42]), (vec![0, 0, 0, 2], vec![42])],
    )]);
    let mut vm = Vm::default();
    let num_keys = 3;
    let num_words = 2;
    let mem_len = num_keys * 2 + num_words;
    let ops = &[
        asm::Stack::Push(mem_len).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(0).into(), // Key1
        asm::Stack::Push(0).into(), // Key2
        asm::Stack::Push(0).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_keys).into(),
        asm::Stack::Push(0).into(), // mem addr
        asm::StateRead::KeyRange,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(vm.memory[..].len(), mem_len as usize);
    assert_eq!(&vm.memory[..], &[6, 1, 7, 0, 7, 1, 42, 42]);
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
    let num_keys = 3;
    let num_words = 3;
    let mem_len = num_keys * 2 + num_words;
    let [addr0, addr1, addr2, addr3] = word_4_from_u8_32(ext_contract_addr.0);
    let ops = &[
        asm::Stack::Push(mem_len).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(addr0).into(),
        asm::Stack::Push(addr1).into(),
        asm::Stack::Push(addr2).into(),
        asm::Stack::Push(addr3).into(),
        asm::Stack::Push(1).into(), // Key0
        asm::Stack::Push(2).into(), // Key1
        asm::Stack::Push(3).into(), // Key2
        asm::Stack::Push(4).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_keys).into(),
        asm::Stack::Push(0).into(), // mem addr
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
    assert_eq!(vm.memory[..].len(), mem_len as usize);
    assert_eq!(&vm.memory[..], &[6, 1, 7, 1, 8, 1, 40, 41, 42]);
}

#[tokio::test]
async fn state_read_ext_nones() {
    let ext_contract_addr = ContentAddress([0x12; 32]);
    let state = State::new(vec![(ext_contract_addr.clone(), vec![])]);
    let mut vm = Vm::default();
    let num_keys = 3;
    let num_words = 0;
    let mem_len = num_keys * 2 + num_words;
    let [addr0, addr1, addr2, addr3] = word_4_from_u8_32(ext_contract_addr.0);
    let ops = &[
        asm::Stack::Push(mem_len).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(addr0).into(),
        asm::Stack::Push(addr1).into(),
        asm::Stack::Push(addr2).into(),
        asm::Stack::Push(addr3).into(),
        asm::Stack::Push(1).into(), // Key0
        asm::Stack::Push(2).into(), // Key1
        asm::Stack::Push(3).into(), // Key2
        asm::Stack::Push(4).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_keys).into(),
        asm::Stack::Push(0).into(), // mem addr
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
    assert_eq!(vm.memory[..].len(), mem_len as usize);
    assert_eq!(&vm.memory[..], &[6, 0, 6, 0, 6, 0]);
}

#[tokio::test]
async fn state_read_various_size_values() {
    let access = *test_access();
    let state = State::new(vec![(
        access.this_data().predicate_to_solve.contract.clone(),
        vec![
            (vec![0, 0, 0, 0], vec![0; 2]),
            (vec![0, 0, 0, 1], vec![1; 22]),
            (vec![0, 0, 0, 2], vec![2; 14]),
            (vec![0, 0, 0, 4], vec![4; 12]),
        ],
    )]);
    let mut vm = Vm::default();
    let num_keys = 5;
    let num_words = 2 + 22 + 14 + 12;
    let mem_len = num_keys * 2 + num_words;
    let ops = &[
        asm::Stack::Push(mem_len).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(0).into(), // Key1
        asm::Stack::Push(0).into(), // Key2
        asm::Stack::Push(0).into(), // Key3
        asm::Stack::Push(4).into(), // key length
        asm::Stack::Push(num_keys).into(),
        asm::Stack::Push(0).into(), // mem addr
        asm::StateRead::KeyRange,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    assert_eq!(vm.memory[..].len(), mem_len as usize);
    let mut expected = vec![10, 2, 12, 22, 34, 14, 48, 0, 48, 12]; // [index, len]s
    expected.append(&mut vec![0; 2]);
    expected.append(&mut vec![1; 22]);
    expected.append(&mut vec![2; 14]);
    expected.append(&mut vec![4; 12]);
    assert_eq!(Vec::from(vm.memory), expected);
}

#[tokio::test]
async fn state_read_various_key_sizes() {
    let access = *test_access();
    let kv_pairs = vec![
        (vec![0], vec![0; 2]),
        (vec![0, 1], vec![7; 6]),
        (vec![1], vec![1; 22]),
        (vec![0, 0, 0, 0, 0, 2], vec![2; 14]),
        (vec![4; 1000], vec![4; 12]),
    ];
    let state = State::new(vec![(
        access.this_data().predicate_to_solve.contract.clone(),
        kv_pairs.clone(),
    )]);
    let mut vm = Vm::default();

    // Total memory length is all the index+len pairs for 5 keys, plus the lengths of the 3 values
    // that actually get read from state.
    let mem_len =
        ((5 * 2) + kv_pairs[0].1.len() + kv_pairs[2].1.len() + kv_pairs[3].1.len()) as Word;

    // The second KeyRange memory address is equal to the length of the first KeyRange read, which
    // is three keys from `[0]` and their values.
    let krng2_mem_addr = ((3 * 2) + kv_pairs[0].1.len() + kv_pairs[2].1.len()) as Word;

    let ops = &[
        asm::Stack::Push(mem_len).into(),
        asm::Memory::Alloc.into(),
        asm::Stack::Push(0).into(), // Key0
        asm::Stack::Push(1).into(), // key length
        asm::Stack::Push(3).into(), // num keys
        asm::Stack::Push(0).into(), // mem addr
        asm::StateRead::KeyRange,
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(6).into(),              // key length
        asm::Stack::Push(2).into(),              // num keys
        asm::Stack::Push(krng2_mem_addr).into(), // mem addr
        asm::StateRead::KeyRange,
        asm::TotalControlFlow::Halt.into(),
    ];
    vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await
        .unwrap();
    // First `KeyRange` result.
    let mut expected = vec![6, 2, 8, 22, 30, 0];
    expected.append(&mut vec![0; 2]);
    expected.append(&mut vec![1; 22]);
    // Second `KeyRange` result.
    let addr = expected.len() as Word + 2 * 2;
    expected.append(&mut vec![addr, 14, addr + 14, 0]);
    expected.append(&mut vec![2; 14]);
    assert_eq!(&vm.memory[..], &expected[..]);
}
