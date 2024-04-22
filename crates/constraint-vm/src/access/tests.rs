use super::*;
use crate::{
    asm,
    error::{AccessError, ConstraintError, OpError},
    eval_ops, exec_ops,
    test_util::*,
};
use essential_types::{
    solution::{DecisionVariableIndex, Mutation, Solution, StateMutation},
    ContentAddress, IntentAddress,
};

#[test]
fn decision_var_inline() {
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                intent_to_solve: TEST_INTENT_ADDR,
                decision_variables: vec![DecisionVariable::Inline(42)],
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
fn decision_var_transient() {
    // Test resolution of transient decision vars over the following path:
    // - Solution 1, Var 2 (start)
    // - Solution 0, Var 3
    // - Solution 2, Var 1
    let access = Access {
        solution: SolutionAccess {
            data: &[
                SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![
                        DecisionVariable::Inline(0),
                        DecisionVariable::Inline(1),
                        DecisionVariable::Inline(2),
                        DecisionVariable::Transient(DecisionVariableIndex {
                            solution_data_index: 2,
                            variable_index: 1,
                        }),
                    ],
                },
                SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![
                        DecisionVariable::Inline(0),
                        DecisionVariable::Inline(1),
                        DecisionVariable::Transient(DecisionVariableIndex {
                            solution_data_index: 0,
                            variable_index: 3,
                        }),
                        DecisionVariable::Inline(3),
                    ],
                },
                SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![
                        DecisionVariable::Inline(0),
                        DecisionVariable::Inline(42),
                    ],
                },
            ],
            // Solution data for intent being solved is at index 1.
            index: 1,
            mutable_keys: test_empty_keys(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(2).into(), // Slot index.
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
                decision_variables: vec![
                    DecisionVariable::Inline(7),
                    DecisionVariable::Inline(8),
                    DecisionVariable::Inline(9),
                ],
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
fn decision_var_range_transient() {
    let access = Access {
        solution: SolutionAccess {
            data: &[
                SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![
                        DecisionVariable::Transient(DecisionVariableIndex {
                            solution_data_index: 1,
                            variable_index: 2,
                        }),
                        DecisionVariable::Transient(DecisionVariableIndex {
                            solution_data_index: 1,
                            variable_index: 1,
                        }),
                        DecisionVariable::Transient(DecisionVariableIndex {
                            solution_data_index: 1,
                            variable_index: 0,
                        }),
                    ],
                },
                SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![
                        DecisionVariable::Inline(7),
                        DecisionVariable::Inline(8),
                        DecisionVariable::Inline(9),
                    ],
                },
            ],
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
    assert_eq!(&stack[..], &[9, 8, 7]);
}

#[test]
fn decision_var_transient_cycle() {
    let access = Access {
        solution: SolutionAccess {
            data: &[
                SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![DecisionVariable::Transient(DecisionVariableIndex {
                        solution_data_index: 1,
                        variable_index: 0,
                    })],
                },
                SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![DecisionVariable::Transient(DecisionVariableIndex {
                        solution_data_index: 0,
                        variable_index: 0,
                    })],
                },
            ],
            index: 0,
            mutable_keys: test_empty_keys(),
        },
        state_slots: StateSlots::EMPTY,
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Access::DecisionVar.into(),
    ];
    let res = exec_ops(ops, access);
    match res {
        Err(ConstraintError::Op(
            _,
            OpError::Access(AccessError::TransientDecisionVariableCycle),
        )) => (),
        _ => panic!("expected transient decision variable cycle error, got {res:?}"),
    }
}

#[test]
fn decision_var_slot_oob() {
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                intent_to_solve: TEST_INTENT_ADDR,
                decision_variables: vec![DecisionVariable::Inline(42)],
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
                    key: [0, 0, 0, 1],
                    value: Some(1),
                }],
            },
            StateMutation {
                pathway: 1,
                mutations: vec![
                    Mutation {
                        key: [1, 1, 1, 1],
                        value: Some(6),
                    },
                    Mutation {
                        key: [1, 1, 1, 2],
                        value: Some(7),
                    },
                ],
            },
            StateMutation {
                pathway: 1,
                mutations: vec![Mutation {
                    key: [2, 2, 2, 1],
                    value: Some(42),
                }],
            },
        ],
        partial_solutions: vec![],
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
            pre: &[Some(0), Some(42)],
            post: &[Some(0), Some(0)],
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
            pre: &[Some(0), Some(0)],
            post: &[Some(42), Some(0)],
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
            pre: &[Some(0), Some(42)],
            post: &[Some(0), Some(0)],
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
            pre: &[Some(0), Some(42)],
            post: &[Some(0), Some(0)],
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
            pre: &[None],
            post: &[None],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(0).into(), // Delta.
        asm::Access::State.into(),
    ];
    let stack = exec_ops(ops, access).unwrap();
    assert_eq!(&stack[..], &[0]);
}

#[test]
fn state_range_pre_mutation() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[Some(10), Some(20), Some(30)],
            post: &[Some(0), Some(0), Some(0)],
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
            pre: &[Some(0), Some(0), Some(0)],
            post: &[Some(0), Some(40), Some(50)],
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
fn state_is_some_pre_mutation_false() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[Some(0), None],
            post: &[Some(0), Some(0)],
        },
    };
    let ops = &[
        asm::Stack::Push(1).into(), // Slot index.
        asm::Stack::Push(0).into(), // Delta (0 for pre-mutation state).
        asm::Access::StateIsSome.into(),
    ];
    // Expect false for `None`.
    assert!(!eval_ops(ops, access).unwrap());
}

#[test]
fn state_is_some_post_mutation_true() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[None, None],
            post: &[Some(42), None],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(1).into(), // Delta (1 for post-mutation state).
        asm::Access::StateIsSome.into(),
    ];
    // Expect true for `Some(42)`.
    assert!(eval_ops(ops, access).unwrap());
}

#[test]
fn state_is_some_range_pre_mutation() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[Some(10), None, Some(30)],
            post: &[None, None, None],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(3).into(), // Range length.
        asm::Stack::Push(0).into(), // Delta (0 for pre-mutation state).
        asm::Access::StateIsSomeRange.into(),
    ];
    let stack = exec_ops(ops, access).unwrap();
    // Expect true, false, true for `Some(10), None, Some(30)`.
    assert_eq!(&stack[..], &[1, 0, 1]);
}

#[test]
fn state_is_some_range_post_mutation() {
    let access = Access {
        solution: *test_solution_access(),
        state_slots: StateSlots {
            pre: &[None, None, None],
            post: &[None, Some(40), None],
        },
    };
    let ops = &[
        asm::Stack::Push(0).into(), // Slot index.
        asm::Stack::Push(3).into(), // Range length.
        asm::Stack::Push(1).into(), // Delta (1 for post-mutation state).
        asm::Access::StateIsSomeRange.into(),
    ];
    let stack = exec_ops(ops, access).unwrap();
    // Expect false, true, false for `None, Some(40), None`.
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

#[test]
fn test_repeat_dec_var() {
    let solution = SolutionAccess {
        data: &[SolutionData {
            intent_to_solve: TEST_INTENT_ADDR,
            decision_variables: vec![DecisionVariable::Inline(3)],
        }],
        index: 0,
        mutable_keys: test_empty_keys(),
    };
    let mut stack = Stack::default();
    let mut repeat = Repeat::new();
    let pc = 0;

    stack.push(0).unwrap();

    repeat_dec_var(solution, &mut stack, &pc, &mut repeat).unwrap();

    assert_eq!(repeat.counter().unwrap(), 3);

    stack.push(1).unwrap();

    repeat_dec_var(solution, &mut stack, &pc, &mut repeat).unwrap_err();

    stack.push(0).unwrap();
    let pc = usize::MAX;

    repeat_dec_var(solution, &mut stack, &pc, &mut repeat).unwrap_err();
}
