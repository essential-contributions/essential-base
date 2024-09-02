use super::*;
use crate::{
    asm,
    error::{AccessError, ConstraintError, OpError},
    eval_ops, exec_ops,
    test_util::*,
    Gas,
};
use essential_constraint_asm::Op;
use essential_types::{
    solution::{Mutation, Solution},
    ContentAddress, PredicateAddress,
};

macro_rules! check_dec_var {
    ($d:expr, $s:expr, $f:ident) => {{
        let d = [SolutionData {
            predicate_to_solve: TEST_PREDICATE_ADDR,
            decision_variables: $d,
            state_mutations: Default::default(),
            transient_data: Default::default(),
        }];
        let access = SolutionAccess {
            data: &d,
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: test_transient_data(),
        };
        $f(access, $s)
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
        OpError::Access(AccessError::DecisionSlotOutOfBounds)
    );

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 42);

    // Get's first word,
    let d = vec![(0..10).collect()];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);

    // Get's first word with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var).unwrap();
    assert_eq!(stack.pop().unwrap(), 10);
}

#[test]
fn test_decision_var_at() {
    let d = vec![vec![42]];

    // Empty stack.
    let mut stack = Stack::default();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_at).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Missing var index
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_at).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(0).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_at).unwrap_err(),
        OpError::Access(AccessError::DecisionSlotOutOfBounds)
    );

    // Index out-of-bounds.
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d, &mut stack, decision_var_at).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_at).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var_at).unwrap();
    assert_eq!(stack.pop().unwrap(), 42);

    // Get's word,
    let d = vec![(0..10).collect()];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(5).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var_at).unwrap();
    assert_eq!(stack.pop().unwrap(), 5);

    // Get's word with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(5).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var_at).unwrap();
    assert_eq!(stack.pop().unwrap(), 15);
}

#[test]
fn test_decision_var_range() {
    let d = vec![vec![42, 43]];

    // Empty stack.
    let mut stack = Stack::default();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_range).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Missing var index
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_range).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Missing len
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_range).unwrap_err(),
        OpError::Stack(StackError::Empty)
    );

    // Slot out-of-bounds.
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_range).unwrap_err(),
        OpError::Access(AccessError::DecisionSlotOutOfBounds)
    );

    // Index out-of-bounds.
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(2).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_range).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Length out-of-bounds.
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(3).unwrap();
    matches!(
        check_dec_var!(d, &mut stack, decision_var_range).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds but value is empty
    let d = vec![vec![]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    matches!(
        check_dec_var!(d.clone(), &mut stack, decision_var_range).unwrap_err(),
        OpError::Access(AccessError::DecisionIndexOutOfBounds)
    );

    // Slot index in-bounds and value is not empty
    let d = vec![vec![42, 43]];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(2).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var_range).unwrap();
    assert_eq!(stack.pop().unwrap(), 43);
    assert_eq!(stack.pop().unwrap(), 42);

    // Get's range,
    let d = vec![(0..10).collect()];
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(5).unwrap();
    stack.push(3).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var_range).unwrap();
    assert_eq!(*stack, vec![5, 6, 7]);

    // Get's word with multiple slots,
    let d = vec![(0..10).collect(), (10..20).collect()];
    let mut stack = Stack::default();
    stack.push(1).unwrap();
    stack.push(5).unwrap();
    stack.push(3).unwrap();
    check_dec_var!(d.clone(), &mut stack, decision_var_range).unwrap();
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
        OpError::Access(AccessError::DecisionSlotOutOfBounds)
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
fn decision_var_ops() {
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                predicate_to_solve: TEST_PREDICATE_ADDR,
                decision_variables: vec![vec![42]],
                state_mutations: Default::default(),
                transient_data: Default::default(),
            }],
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: test_transient_data(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Access::DecisionVar.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[42]);
}

#[test]
fn decision_var_range_ops() {
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                predicate_to_solve: TEST_PREDICATE_ADDR,
                decision_variables: vec![vec![7, 8, 9], vec![10, 11, 12]],
                state_mutations: Default::default(),
                transient_data: Default::default(),
            }],
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: test_transient_data(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot.
        asm::Stack::Push(0).into(), // Index.
        asm::Stack::Push(3).into(), // Range length.
        asm::Access::DecisionVarRange.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[7, 8, 9]);
}

#[test]
fn decision_var_slot_oob_ops() {
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                predicate_to_solve: TEST_PREDICATE_ADDR,
                decision_variables: vec![vec![42]],
                state_mutations: Default::default(),
                transient_data: Default::default(),
            }],
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: test_transient_data(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(1).into(), // Slot index.
        asm::Access::DecisionVar.into(),
    ];
    let res = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX);
    match res {
        Err(ConstraintError::Op(_, OpError::Access(AccessError::DecisionSlotOutOfBounds))) => {}
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
                transient_data: Default::default(),
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
                transient_data: Default::default(),
            },
        ],
        // All state mutations, 3 of which point to the predicate we're solving.
    };

    // The predicate we're solving is the second predicate, i.e. index `1`.
    let predicate_index = 1;

    let mutable_keys = mut_keys_set(&solution, predicate_index);

    // Construct access to the parts of the solution that we need for checking.
    let access = Access {
        solution: SolutionAccess::new(
            &solution,
            predicate_index,
            &mutable_keys,
            test_transient_data(),
        ),
        state_slots: StateSlots::EMPTY,
    };

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
    let stack = exec_ops(&ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[1]);
}

#[test]
fn state_pre_mutation() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![0], vec![42]],
            post: &[vec![0], vec![0]],
        },
    };
    let ops = &[
        asm::Stack::Push(1).into(), // Slot index.
        asm::Stack::Push(0).into(), // Delta (0 for pre-mutation state).
        asm::Access::State.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[42]);
}

#[test]
fn state_post_mutation() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![0], vec![0]],
            post: &[vec![42], vec![0]],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(1).into(), // Delta (1 for post-mutation state).
        asm::Access::State.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[42]);
}

#[test]
fn state_pre_mutation_oob() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![0], vec![42]],
            post: &[vec![0], vec![0]],
        },
    };
    let ops = &[
        asm::Stack::Push(2).into(), // Slot index (out-of-bounds).
        asm::Stack::Push(0).into(), // Delta (0 for pre-mutation state).
        asm::Access::State.into(),
    ];
    let res = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX);
    match res {
        Err(ConstraintError::Op(_, OpError::Access(AccessError::StateSlotOutOfBounds))) => (),
        _ => panic!("expected state slot out-of-bounds error, got {res:?}"),
    }
}

#[test]
fn invalid_state_slot_delta() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![0], vec![42]],
            post: &[vec![0], vec![0]],
        },
    };
    let ops = &[
        asm::Stack::Push(1).into(), // Slot index.
        asm::Stack::Push(2).into(), // Delta (invalid).
        asm::Access::State.into(),
    ];
    let res = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX);
    match res {
        Err(ConstraintError::Op(_, OpError::Access(AccessError::InvalidStateSlotDelta(2)))) => {}
        _ => panic!("expected invalid state slot delta error, got {res:?}"),
    }
}

#[test]
fn state_slot_was_none() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![]],
            post: &[vec![]],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(0).into(), // Delta.
        asm::Access::State.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert!(&stack.is_empty());
}

#[test]
fn state_range_pre_mutation() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![10], vec![20], vec![30]],
            post: &[vec![0], vec![0], vec![0]],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(3).into(), // Range length.
        asm::Stack::Push(0).into(), // Delta (0 for pre-mutation state).
        asm::Access::StateRange.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[10, 20, 30]);
}

#[test]
fn state_range_post_mutation() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![0], vec![0], vec![0]],
            post: &[vec![0], vec![40], vec![50]],
        },
    };
    let ops = &[
        asm::Stack::Push(1).into(), // Slot index.
        asm::Stack::Push(2).into(), // Range length.
        asm::Stack::Push(1).into(), // Delta (1 for post-mutation state).
        asm::Access::StateRange.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[40, 50]);
}

#[test]
fn state_is_not_empty_pre_mutation_false() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![0], vec![]],
            post: &[vec![0], vec![0]],
        },
    };
    let ops = &[
        asm::Stack::Push(1).into(), // Slot index.
        asm::Stack::Push(0).into(), // Delta (0 for pre-mutation state).
        asm::Access::StateLen.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
    ];
    // Expect false for `vec![]`.
    assert!(!eval_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap());
}

#[test]
fn state_is_not_empty_post_mutation_true() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![], vec![]],
            post: &[vec![42], vec![]],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(1).into(), // Delta (1 for post-mutation state).
        asm::Access::StateLen.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
    ];
    // Expect true for `vec![42]`.
    assert!(eval_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap());
}

#[test]
fn state_is_some_range_pre_mutation() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![10], vec![], vec![30]],
            post: &[vec![], vec![], vec![]],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(3).into(), // Range length.
        asm::Stack::Push(0).into(), // Delta (0 for pre-mutation state).
        asm::Access::StateLenRange.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    // Expect true, false, true for `vec![10], vec![], vec![30]`.
    assert_eq!(&stack[..], &[1, 0, 1]);
}

#[test]
fn state_is_some_range_post_mutation() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[vec![], vec![], vec![]],
            post: &[vec![], vec![40], vec![]],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(3).into(), // Range length.
        asm::Stack::Push(1).into(), // Delta (1 for post-mutation state).
        asm::Access::StateLenRange.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    // Expect false, true, false for `vec![], vec![40], vec![]`.
    assert_eq!(&stack[..], &[0, 1, 0]);
}

#[test]
fn this_address() {
    let ops = &[asm::Access::ThisAddress.into()];
    let stack = exec_ops(ops, *test_access(), &|_: &Op| 1, Gas::MAX).unwrap();
    let expected_words = word_4_from_u8_32(TEST_PREDICATE_ADDR.predicate.0);
    assert_eq!(&stack[..], expected_words);
}

#[test]
fn this_contract_address() {
    let ops = &[asm::Access::ThisContractAddress.into()];
    let stack = exec_ops(ops, *test_access(), &|_: &Op| 1, Gas::MAX).unwrap();
    let expected_words = word_4_from_u8_32(TEST_PREDICATE_ADDR.contract.0);
    assert_eq!(&stack[..], expected_words);
}

#[test]
fn transient() {
    let transient_data = [(0, [(vec![3], vec![2])].into_iter().collect())]
        .into_iter()
        .collect();
    let access = Access {
        solution: SolutionAccess {
            data: test_solution_data_arr(),
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: &transient_data,
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(3).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::Access::Transient.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[2]);
}

#[test]
fn transient_len() {
    let transient_data = [(0, [(vec![3], vec![2])].into_iter().collect())]
        .into_iter()
        .collect();
    let access = Access {
        solution: SolutionAccess {
            data: test_solution_data_arr(),
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: &transient_data,
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(3).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::Access::TransientLen.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[1]);
}

#[test]
fn predicate_at() {
    let transient_data = [(0, [(vec![3], vec![2])].into_iter().collect())]
        .into_iter()
        .collect();
    let data = [SolutionData {
        predicate_to_solve: TEST_PREDICATE_ADDR,
        decision_variables: vec![],
        state_mutations: vec![],
        transient_data: vec![],
    }];
    let access = Access {
        solution: SolutionAccess {
            data: &data,
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: &transient_data,
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[asm::Stack::Push(0).into(), asm::Access::PredicateAt.into()];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    let predicate = data[0].predicate_to_solve.clone();
    let mut expected = vec![];
    expected.extend(word_4_from_u8_32(predicate.contract.0));
    expected.extend(word_4_from_u8_32(predicate.predicate.0));
    assert_eq!(&stack[..], expected);
}

#[test]
fn this_transient_len() {
    let transient_data = [(0, [(vec![3], vec![2])].into_iter().collect())]
        .into_iter()
        .collect();
    let data = [SolutionData {
        predicate_to_solve: TEST_PREDICATE_ADDR,
        decision_variables: vec![],
        state_mutations: vec![],
        transient_data: vec![],
    }];
    let access = Access {
        solution: SolutionAccess {
            data: &data,
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: &transient_data,
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[asm::Access::ThisTransientLen.into()];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[1]);
}

#[test]
fn this_transient_contains() {
    let transient_data = [(0, [(vec![3], vec![2])].into_iter().collect())]
        .into_iter()
        .collect();
    let data = [SolutionData {
        predicate_to_solve: TEST_PREDICATE_ADDR,
        decision_variables: vec![],
        state_mutations: vec![],
        transient_data: vec![],
    }];
    let access = Access {
        solution: SolutionAccess {
            data: &data,
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: &transient_data,
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(3).into(),
        asm::Stack::Push(1).into(),
        asm::Access::ThisTransientContains.into(),
    ];
    let stack = exec_ops(ops, access, &|_: &Op| 1, Gas::MAX).unwrap();
    assert_eq!(&stack[..], &[1]);
}
