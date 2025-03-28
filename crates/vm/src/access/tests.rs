use super::*;
use crate::{
    asm,
    error::{AccessError, ExecError, OpError},
    sync::{exec_ops, test_util::*},
    types::solution::Solution,
    utils::EmptyState,
    GasLimit, Op,
};

macro_rules! check_dec_var {
    ($d:expr, $s:expr, $f:ident) => {{
        $f(&$d, $s)
    }};
}

#[test]
fn test_predicate_data() {
    let d = vec![vec![42]];

    // Empty stack.
    let mut stack = Stack::default();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::MissingArg(MissingAccessArgError::PredDataLen))
    ));

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    assert!(matches!(
        check_dec_var!(d, &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::MissingArg(
            MissingAccessArgError::PredDataValueIx
        ))
    ));

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::PredicateDataValueRangeOutOfBounds(0, 1))
    ));

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert_eq!(stack.pop().unwrap(), 42);

    // Get's first word,
    let d = vec![(0..10).collect()];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);

    // Get's first word with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert_eq!(stack.pop().unwrap(), 10);
}

#[test]
fn test_predicate_data_at() {
    let d = vec![vec![42], vec![9, 20]];

    // Missing pred data len.
    let mut stack = Stack::default();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::MissingArg(MissingAccessArgError::PredDataLen))
    ));

    // Missing value index
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::MissingArg(
            MissingAccessArgError::PredDataValueIx
        ))
    ));

    // Missing slot ix
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::MissingArg(
            MissingAccessArgError::PredDataSlotIx
        ))
    ));

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(2).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::PredicateDataSlotIxOutOfBounds(_))
    ));

    // Index out-of-bounds.
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    stack.push(1).unwrap();
    assert!(matches!(
        check_dec_var!(d, &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::PredicateDataValueRangeOutOfBounds(_, _))
    ));

    // Value range out of bounds.
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    assert!(matches!(
        check_dec_var!(d, &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::PredicateDataValueRangeOutOfBounds(_, _))
    ));

    // Slot index in-bounds, value is empty and length is 0
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert!(stack.is_empty());

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert_eq!(stack.pop().unwrap(), 42);

    // Get's word,
    let d = vec![(0..10).collect()];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(5).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert_eq!(stack.pop().unwrap(), 5);

    // Get's word with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(5).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert_eq!(stack.pop().unwrap(), 15);
}

#[test]
fn test_predicate_data_range() {
    let d = vec![vec![42, 43], vec![44, 45, 46]];

    // Missing len.
    let mut stack = Stack::default();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::MissingArg(MissingAccessArgError::PredDataLen))
    ));

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(2).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::PredicateDataSlotIxOutOfBounds(_))
    ));

    // Value range out-of-bounds.
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(2).unwrap();
    stack.push(1).unwrap();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::PredicateDataValueRangeOutOfBounds(_, _))
    ));

    // Length out-of-bounds.
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(3).unwrap();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::PredicateDataValueRangeOutOfBounds(_, _))
    ));

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    assert!(matches!(
        check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap_err(),
        OpError::Access(AccessError::PredicateDataValueRangeOutOfBounds(_, _))
    ));

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42, 43]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(2).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert_eq!(stack.pop().unwrap(), 43);
    assert_eq!(stack.pop().unwrap(), 42);

    // Get's range,
    let d = vec![(0..10).collect()];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(5).unwrap();
    stack.push(3).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert_eq!(*stack, vec![5, 6, 7]);

    // Get's word with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(5).unwrap();
    stack.push(3).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data).unwrap();
    assert_eq!(*stack, vec![15, 16, 17]);
}

#[test]
fn test_predicate_data_len() {
    let d = vec![vec![42, 43]];

    // Empty stack.
    let mut stack = Stack::default();
    assert!(matches!(
        predicate_data_len(&d.clone(), &mut stack).unwrap_err(),
        AccessError::MissingArg(MissingAccessArgError::PredDataSlotIx),
    ));

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    assert!(matches!(
        predicate_data_len(&d.clone(), &mut stack).unwrap_err(),
        AccessError::PredicateDataSlotIxOutOfBounds(_)
    ));

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data_len).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42, 43]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data_len).unwrap();
    assert_eq!(stack.pop().unwrap(), 2);

    // Get's length with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, predicate_data_len).unwrap();
    assert_eq!(stack.pop().unwrap(), 10);
}

#[test]
fn predicate_data_single_word_ops() {
    let access = Access {
        solutions: Arc::new(vec![Solution {
            predicate_to_solve: TEST_PREDICATE_ADDR,
            predicate_data: vec![vec![42]],
            state_mutations: Default::default(),
        }]),
        index: 0,
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(0).into(), // Value index.
        asm::Stack::Push(1).into(), // Length.
        asm::Access::PredicateData.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let stack = exec_ops(ops, access, &EmptyState, op_gas_cost, GasLimit::UNLIMITED).unwrap();
    assert_eq!(&stack[..], &[42]);
}

#[test]
fn predicate_data_ops() {
    let access = Access {
        solutions: Arc::new(vec![Solution {
            predicate_to_solve: TEST_PREDICATE_ADDR,
            predicate_data: vec![vec![7, 8, 9], vec![10, 11, 12]],
            state_mutations: Default::default(),
        }]),
        index: 0,
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot.
        asm::Stack::Push(0).into(), // Index.
        asm::Stack::Push(3).into(), // Range length.
        asm::Access::PredicateData.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let stack = exec_ops(ops, access, &EmptyState, op_gas_cost, GasLimit::UNLIMITED).unwrap();
    assert_eq!(&stack[..], &[7, 8, 9]);
}

#[test]
fn predicate_data_slot_oob_ops() {
    let access = Access {
        solutions: Arc::new(vec![Solution {
            predicate_to_solve: TEST_PREDICATE_ADDR,
            predicate_data: vec![vec![42]],
            state_mutations: Default::default(),
        }]),
        index: 0,
    };
    let ops = &[
        asm::Stack::Push(1).into(), // Slot index.
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::Access::PredicateData.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let res = exec_ops(ops, access, &EmptyState, op_gas_cost, GasLimit::UNLIMITED);
    match res {
        Err(ExecError(_, OpError::Access(AccessError::PredicateDataSlotIxOutOfBounds(_)))) => {}
        _ => panic!("expected predicate data slot out-of-bounds error, got {res:?}"),
    }
}

#[test]
fn this_address() {
    let ops = &[asm::Access::ThisAddress.into()];
    let op_gas_cost = &|_: &Op| 1;
    let stack = exec_ops(
        ops,
        test_access().clone(),
        &EmptyState,
        op_gas_cost,
        GasLimit::UNLIMITED,
    )
    .unwrap();
    let expected_words = word_4_from_u8_32(TEST_PREDICATE_ADDR.predicate.0);
    assert_eq!(&stack[..], expected_words);
}

#[test]
fn this_contract_address() {
    let ops = &[asm::Access::ThisContractAddress.into()];
    let op_gas_cost = &|_: &Op| 1;
    let stack = exec_ops(
        ops,
        test_access().clone(),
        &EmptyState,
        op_gas_cost,
        GasLimit::UNLIMITED,
    )
    .unwrap();
    let expected_words = word_4_from_u8_32(TEST_PREDICATE_ADDR.contract.0);
    assert_eq!(&stack[..], expected_words);
}
