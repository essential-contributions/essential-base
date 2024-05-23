use super::*;
use crate::{
    asm,
    error::{AccessError, ConstraintError, OpError},
    eval_ops, exec_ops,
    test_util::*,
};
use essential_types::{
    solution::{Mutation, Solution, StateMutation},
    ContentAddress, IntentAddress,
};

#[test]
fn decision_var_inline() {
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                intent_to_solve: TEST_INTENT_ADDR,
                decision_variables: vec![42],
            }],
            index: 0,
            mutable_keys: test_empty_keys(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Access::DecisionVar.into(),
    ];
    let stack = exec_ops(ops, access).unwrap();
    assert_eq!(&stack[..], &[42]);
}

#[test]
fn decision_var_range() {
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                intent_to_solve: TEST_INTENT_ADDR,
                decision_variables: vec![7, 8, 9],
            }],
            index: 0,
            mutable_keys: test_empty_keys(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(3).into(), // Range length.
        asm::Access::DecisionVarRange.into(),
    ];
    let stack = exec_ops(ops, access).unwrap();
    assert_eq!(&stack[..], &[7, 8, 9]);
}

#[test]
fn decision_var_slot_oob() {
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                intent_to_solve: TEST_INTENT_ADDR,
                decision_variables: vec![42],
            }],
            index: 0,
            mutable_keys: test_empty_keys(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(1).into(), // Slot index.
        asm::Access::DecisionVar.into(),
    ];
    let res = exec_ops(ops, access);
    match res {
        Err(ConstraintError::Op(_, OpError::Access(AccessError::DecisionSlotOutOfBounds))) => {}
        _ => panic!("expected decision variable slot out-of-bounds error, got {res:?}"),
    }
}

#[test]
fn mut_keys_len() {
    // The intent that we're checking.
    let intent_addr = TEST_INTENT_ADDR;

    // An example solution with some state mutations proposed for the intent
    // at index `1`.
    let solution = Solution {
        data: vec![
            // Solution data for some other intent.
            SolutionData {
                intent_to_solve: IntentAddress {
                    set: ContentAddress([0x13; 32]),
                    intent: ContentAddress([0x31; 32]),
                },
                decision_variables: vec![],
            },
            // Solution data for the intent we're checking.
            SolutionData {
                intent_to_solve: intent_addr.clone(),
                decision_variables: vec![],
            },
        ],
        // All state mutations, 3 of which point to the intent we're solving.
        state_mutations: vec![
            StateMutation {
                pathway: 0,
                mutations: vec![Mutation {
                    key: vec![0, 0, 0, 1],
                    value: vec![1],
                }],
            },
            StateMutation {
                pathway: 1,
                mutations: vec![
                    Mutation {
                        key: vec![1, 1, 1, 1],
                        value: vec![6],
                    },
                    Mutation {
                        key: vec![1, 1, 1, 2],
                        value: vec![7],
                    },
                ],
            },
            StateMutation {
                pathway: 1,
                mutations: vec![Mutation {
                    key: vec![2, 2, 2, 1],
                    value: vec![42],
                }],
            },
        ],
    };

    // The intent we're solving is the second intent, i.e. index `1`.
    let intent_index = 1;

    let mutable_keys = mut_keys_set(&solution, intent_index);

    // Construct access to the parts of the solution that we need for checking.
    let access = Access {
        solution: SolutionAccess::new(&solution, intent_index, &mutable_keys),
        state_slots: StateSlots::EMPTY,
    };

    // Check that there are actually 3 mutations.
    let expected_mut_keys_len = 3;

    // We're only going to execute the `MutKeysLen` op to check the expected value.
    let ops = &[asm::Access::MutKeysLen.into()];
    let stack = exec_ops(ops, access).unwrap();
    assert_eq!(&stack[..], &[expected_mut_keys_len]);
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
    let stack = exec_ops(ops, access).unwrap();
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
    let stack = exec_ops(ops, access).unwrap();
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
    let res = exec_ops(ops, access);
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
    let res = exec_ops(ops, access);
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
    let stack = exec_ops(ops, access).unwrap();
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
    let stack = exec_ops(ops, access).unwrap();
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
    let stack = exec_ops(ops, access).unwrap();
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
    assert!(!eval_ops(ops, access).unwrap());
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
    assert!(eval_ops(ops, access).unwrap());
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
    let stack = exec_ops(ops, access).unwrap();
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
    let stack = exec_ops(ops, access).unwrap();
    // Expect false, true, false for `vec![], vec![40], vec![]`.
    assert_eq!(&stack[..], &[0, 1, 0]);
}

#[test]
fn this_address() {
    let ops = &[asm::Access::ThisAddress.into()];
    let stack = exec_ops(ops, *test_access()).unwrap();
    let expected_words = word_4_from_u8_32(TEST_INTENT_ADDR.intent.0);
    assert_eq!(&stack[..], expected_words);
}

#[test]
fn this_set_address() {
    let ops = &[asm::Access::ThisSetAddress.into()];
    let stack = exec_ops(ops, *test_access()).unwrap();
    let expected_words = word_4_from_u8_32(TEST_INTENT_ADDR.set.0);
    assert_eq!(&stack[..], expected_words);
}
