use super::*;
use crate::error::StackError;
use crate::{
    asm,
    error::{AccessError, ConstraintError, OpError},
    exec_ops,
    test_util::*,
};
use essential_types::{
    solution::{Mutation, Solution},
    ContentAddress, PredicateAddress,
};

macro_rules! check_dec_var {
    ($d:expr, $s:expr, $f:ident) => {{
        $f(&$d, $s)
    }};
}

#[test]
fn test_decision_var() {
    let d = vec![vec![42]];

    // Empty stack.
    let mut stack = Stack::default();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d, &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionSlotIxOutOfBounds(_))
    );

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 42);

    // Get's first word,
    let d = vec![(0..10).collect()];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);

    // Get's first word with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 10);
}

#[test]
fn test_decision_var_at() {
    let d = vec![vec![42], vec![9, 20]];

    // Empty stack.
    let mut stack = Stack::default();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Missing value index
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Missing length
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(2).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionSlotIxOutOfBounds(_))
    );

    // Index out-of-bounds.
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d, &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds, value is empty and length is 0
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert!(stack.is_empty());

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 42);

    // Get's word,
    let d = vec![(0..10).collect()];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(5).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 5);

    // Get's word with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(5).unwrap();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 15);
}

#[test]
fn test_decision_var_range() {
    let d = vec![vec![42, 43], vec![44, 45, 46]];

    // Empty stack.
    let mut stack = Stack::default();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(2).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionSlotIxOutOfBounds(_))
    );

    // Index out-of-bounds.
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(2).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Length out-of-bounds.
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(3).unwrap();
    matches!(
        check_dec_var!(d, &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42, 43]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(2).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 43);
    assert_eq!(stack.pop().unwrap(), 42);

    // Get's range,
    let d = vec![(0..10).collect()];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(5).unwrap();
    stack.push(3).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(*stack, vec![5, 6, 7]);

    // Get's word with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(5).unwrap();
    stack.push(3).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(*stack, vec![15, 16, 17]);
}

#[test]
fn test_decision_var_len() {
    let d = vec![vec![42, 43]];

    // Empty stack.
    let mut stack = Stack::default();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_len).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_len).unwrap_err(),
        OpError::Access(AccessError::DecisionSlotIxOutOfBounds(_))
    );

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var_len).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42, 43]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var_len).unwrap();
    assert_eq!(stack.pop().unwrap(), 2);

    // Get's length with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var_len).unwrap();
    assert_eq!(stack.pop().unwrap(), 10);
}

#[test]
fn decision_var_single_word_ops() {
    let access = Access {
        data: &[SolutionData {
            predicate_to_solve: TEST_PREDICATE_ADDR,
            decision_variables: vec![vec![42]],
            state_mutations: Default::default(),
        }],
        index: 0,
        mutable_keys: test_empty_keys(),
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(0).into(), // Value index.
        asm::Stack::Push(1).into(), // Length.
        asm::Access::DecisionVar.into(),
    ];
    let stack = exec_ops(ops, access).unwrap();
    assert_eq!(&stack[..], &[42]);
}

#[test]
fn decision_var_ops() {
    let access = Access {
        data: &[SolutionData {
            predicate_to_solve: TEST_PREDICATE_ADDR,
            decision_variables: vec![vec![7, 8, 9], vec![10, 11, 12]],
            state_mutations: Default::default(),
        }],
        index: 0,
        mutable_keys: test_empty_keys(),
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot.
        asm::Stack::Push(0).into(), // Index.
        asm::Stack::Push(3).into(), // Range length.
        asm::Access::DecisionVar.into(),
    ];
    let stack = exec_ops(ops, access).unwrap();
    assert_eq!(&stack[..], &[7, 8, 9]);
}

#[test]
fn decision_var_slot_oob_ops() {
    let access = Access {
        data: &[SolutionData {
            predicate_to_solve: TEST_PREDICATE_ADDR,
            decision_variables: vec![vec![42]],
            state_mutations: Default::default(),
        }],
        index: 0,
        mutable_keys: test_empty_keys(),
    };
    let ops = &[
        asm::Stack::Push(1).into(), // Slot index.
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::Access::DecisionVar.into(),
    ];
    let res = exec_ops(ops, access);
    match res {
        Err(ConstraintError::Op(_, OpError::Access(AccessError::DecisionSlotIxOutOfBounds(_)))) => {
        }
        _ => panic!("expected decision variable slot out-of-bounds error, got {res:?}"),
    }
}

#[test]
fn mut_keys_push_eq() {
    // The predicate that we're checking.
    let predicate_addr = TEST_PREDICATE_ADDR;

    // An example solution with some state mutations proposed for the predicate
    // at index `1`.
    let solution = Solution {
        data: vec![
            // Solution data for some other predicate.
            SolutionData {
                predicate_to_solve: PredicateAddress {
                    contract: ContentAddress([0x13; 32]),
                    predicate: ContentAddress([0x31; 32]),
                },
                decision_variables: vec![],
                state_mutations: vec![Mutation {
                    key: vec![0, 0, 0, 1],
                    value: vec![1],
                }],
            },
            // Solution data for the predicate we're checking.
            SolutionData {
                predicate_to_solve: predicate_addr.clone(),
                decision_variables: vec![],
                state_mutations: vec![
                    Mutation {
                        key: vec![1, 1, 1, 1],
                        value: vec![6],
                    },
                    Mutation {
                        key: vec![1, 1, 1, 2],
                        value: vec![7],
                    },
                    Mutation {
                        key: vec![2, 2, 2, 1],
                        value: vec![42],
                    },
                ],
            },
        ],
        // All state mutations, 3 of which point to the predicate we're solving.
    };

    // The predicate we're solving is the second predicate, i.e. index `1`.
    let predicate_index = 1;

    let mutable_keys = mut_keys_set(&solution, predicate_index);

    // Construct access to the parts of the solution that we need for checking.
    let access = Access::new(&solution, predicate_index, &mutable_keys);

    // We're only going to execute the `MutKeysLen` op to check the expected value.
    let mut expected_set = vec![];
    for key in solution.data[predicate_index as usize]
        .state_mutations
        .iter()
        .map(|m| &m.key)
    {
        expected_set.extend(key.iter().copied());
        expected_set.push(key.len() as Word);
    }
    expected_set.push(expected_set.len() as Word);

    let mut ops = expected_set
        .into_iter()
        .map(asm::Stack::Push)
        .map(Into::into)
        .collect::<Vec<_>>();

    ops.push(asm::Access::MutKeys.into());
    ops.push(asm::Pred::EqSet.into());
    let stack = exec_ops(&ops, access).unwrap();
    assert_eq!(&stack[..], &[1]);
}

#[test]
fn this_address() {
    let ops = &[asm::Access::ThisAddress.into()];
    let stack = exec_ops(ops, *test_access()).unwrap();
    let expected_words = word_4_from_u8_32(TEST_PREDICATE_ADDR.predicate.0);
    assert_eq!(&stack[..], expected_words);
}

#[test]
fn this_contract_address() {
    let ops = &[asm::Access::ThisContractAddress.into()];
    let stack = exec_ops(ops, *test_access()).unwrap();
    let expected_words = word_4_from_u8_32(TEST_PREDICATE_ADDR.contract.0);
    assert_eq!(&stack[..], expected_words);
}
