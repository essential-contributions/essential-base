//! Access operation implementations.

use std::collections::HashSet;

use crate::{error::AccessError, types::convert::bool_from_word, OpResult, Stack};
use essential_constraint_asm::Word;
use essential_types::{
    convert::word_4_from_u8_32,
    solution::{DecisionVariable, Solution, SolutionData, SolutionDataIndex},
    Key,
};

/// All necessary solution data and state access required to check an individual intent.
#[derive(Clone, Copy, Debug)]
pub struct Access<'a> {
    /// All necessary solution data access required to check an individual intent.
    pub solution: SolutionAccess<'a>,
    /// The pre and post mutation state slot values for the intent being solved.
    pub state_slots: StateSlots<'a>,
}

/// All necessary solution data access required to check an individual intent.
#[derive(Clone, Copy, Debug)]
pub struct SolutionAccess<'a> {
    /// The input data for each intent being solved within the solution.
    ///
    /// We require *all* intent solution data in order to handle transient
    /// decision variable access.
    pub data: &'a [SolutionData],
    /// Checking is performed for one intent at a time. This index refers to
    /// the checked intent's associated solution data within `data`.
    pub index: usize,
    /// The keys being proposed for mutation for the intent.
    pub mutable_keys: &'a HashSet<&'a [Word]>,
}

/// The pre and post mutation state slot values for the intent being solved.
#[derive(Clone, Copy, Debug)]
pub struct StateSlots<'a> {
    /// Intent state slot values before the solution's mutations are applied.
    pub pre: &'a StateSlotSlice,
    /// Intent state slot values after the solution's mutations are applied.
    pub post: &'a StateSlotSlice,
}

/// The state slots declared within the intent.
pub type StateSlotSlice = [Option<Word>];

impl<'a> SolutionAccess<'a> {
    /// A shorthand for constructing a `SolutionAccess` instance for checking
    /// the intent at the given index within the given solution.
    ///
    /// This constructor assumes that the given mutable keys set is correct
    /// for this solution. It is not checked by this function for performance.
    pub fn new(
        solution: &'a Solution,
        intent_index: SolutionDataIndex,
        mutable_keys: &'a HashSet<&[Word]>,
    ) -> Self {
        Self {
            data: &solution.data,
            index: intent_index.into(),
            mutable_keys,
        }
    }

    /// The solution data associated with the intent currently being checked.
    ///
    /// **Panics** in the case that `self.index` is out of range of the `self.data` slice.
    pub fn this_data(&self) -> &SolutionData {
        self.data
            .get(self.index)
            .expect("intent index out of range of solution data")
    }
}

impl<'a> StateSlots<'a> {
    /// Empty state slots.
    pub const EMPTY: Self = Self {
        pre: &[],
        post: &[],
    };
}

/// A helper for collecting all mutable keys that are proposed for mutation for
/// the intent at the given index.
///
/// Specifically, assists in calculating the `mut_keys_len` for
/// `SolutionAccess`, as this is equal to the `.count()` of the returned iterator.
///
/// **Note:** In the case that the given solution is invalid and contains multiple
/// mutations to the same key, the same key will be yielded multiple times.
pub fn mut_keys(
    solution: &Solution,
    intent_index: SolutionDataIndex,
) -> impl Iterator<Item = &Key> {
    solution
        .state_mutations
        .iter()
        .filter(move |state_mutation| state_mutation.pathway == intent_index)
        .flat_map(|state_mutation| state_mutation.mutations.iter().map(|m| &m.key))
}

/// Get the mutable keys as slices
pub fn mut_keys_slices(
    solution: &Solution,
    intent_index: SolutionDataIndex,
) -> impl Iterator<Item = &[Word]> {
    solution
        .state_mutations
        .iter()
        .filter(move |state_mutation| state_mutation.pathway == intent_index)
        .flat_map(|state_mutation| state_mutation.mutations.iter().map(|m| m.key.as_ref()))
}

/// Get the set of mutable keys for this intent.
pub fn mut_keys_set(solution: &Solution, intent_index: SolutionDataIndex) -> HashSet<&[Word]> {
    mut_keys_slices(solution, intent_index).collect()
}

/// `Access::DecisionVar` implementation.
pub(crate) fn decision_var(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    stack.pop1_push1(|slot| {
        let ix = usize::try_from(slot).map_err(|_| AccessError::DecisionSlotOutOfBounds)?;
        let w = resolve_decision_var(solution.data, solution.index, ix)?;
        Ok(w)
    })
}

/// `Access::DecisionVarRange` implementation.
pub(crate) fn decision_var_range(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    let [slot, len] = stack.pop2()?;
    let range = range_from_start_len(slot, len).ok_or(AccessError::DecisionSlotOutOfBounds)?;
    for dec_var_ix in range {
        let w = resolve_decision_var(solution.data, solution.index, dec_var_ix)?;
        stack.push(w)?;
    }
    Ok(())
}

/// `Access::MutKeysLen` implementation.
pub(crate) fn mut_keys_len(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    stack.push(
        solution
            .mutable_keys
            .len()
            .try_into()
            .map_err(|_| AccessError::SolutionDataOutOfBounds)?,
    )?;
    Ok(())
}

pub(crate) fn mut_keys_contains(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    let found = stack.pop_len_words::<_, bool, crate::error::OpError>(|words| {
        Ok(solution.mutable_keys.contains(words))
    })?;
    stack.push(Word::from(found))?;
    Ok(())
}

/// `Access::State` implementation.
pub(crate) fn state(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    stack.pop2_push1(|slot, delta| {
        let slot = state_slot(slots, slot, delta)?;
        let word = slot.unwrap_or_default();
        Ok(word)
    })
}

/// `Access::StateRange` implementation.
pub(crate) fn state_range(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    let [slot, len, delta] = stack.pop3()?;
    let slice = state_slot_range(slots, slot, len, delta)?;
    for slot in slice {
        let word = slot.unwrap_or_default();
        stack.push(word)?;
    }
    Ok(())
}

/// `Access::StateIsSome` implementation.
pub(crate) fn state_is_some(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    stack.pop2_push1(|slot, delta| {
        let slot = state_slot(slots, slot, delta)?;
        let is_some = Word::from(slot.is_some());
        Ok(is_some)
    })
}

/// `Access::StateIsSomeRange` implementation.
pub(crate) fn state_is_some_range(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    let [slot, len, delta] = stack.pop3()?;
    let slice = state_slot_range(slots, slot, len, delta)?;
    for slot in slice {
        let is_some = Word::from(slot.is_some());
        stack.push(is_some)?;
    }
    Ok(())
}

/// `Access::ThisAddress` implementation.
pub(crate) fn this_address(data: &SolutionData, stack: &mut Stack) -> OpResult<()> {
    let words = word_4_from_u8_32(data.intent_to_solve.intent.0);
    stack.extend(words)?;
    Ok(())
}

/// `Access::ThisSetAddress` implementation.
pub(crate) fn this_set_address(data: &SolutionData, stack: &mut Stack) -> OpResult<()> {
    let words = word_4_from_u8_32(data.intent_to_solve.set.0);
    stack.extend(words)?;
    Ok(())
}

/// `Access::ThisPathway` implementation.
pub(crate) fn this_pathway(index: usize, stack: &mut Stack) -> OpResult<()> {
    let index: Word = index
        .try_into()
        .map_err(|_| AccessError::SolutionDataOutOfBounds)?;
    Ok(stack.push(index)?)
}

/// Resolve the decision variable by traversing any necessary transient data.
///
/// Errors if the solution data or decision var indices are out of bounds
/// (whether provided directly or via a transient decision var) or if a cycle
/// occurs between transient decision variables.
fn resolve_decision_var(
    data: &[SolutionData],
    mut data_ix: usize,
    mut var_ix: usize,
) -> Result<Word, AccessError> {
    // Track visited vars `(data_ix, var_ix)` to ensure we do not enter a cycle.
    let mut visited = std::collections::HashSet::new();
    loop {
        let solution_data = data
            .get(data_ix)
            .ok_or(AccessError::SolutionDataOutOfBounds)?;
        let dec_var = solution_data
            .decision_variables
            .get(var_ix)
            .ok_or(AccessError::DecisionSlotOutOfBounds)?;
        match *dec_var {
            DecisionVariable::Inline(w) => return Ok(w),
            DecisionVariable::Transient(ref transient) => {
                // We're traversing transient data, so make sure we track vars already visited.
                if !visited.insert((data_ix, var_ix)) {
                    return Err(AccessError::TransientDecisionVariableCycle);
                }
                data_ix = transient.solution_data_index.into();
                var_ix = transient.variable_index.into();
            }
        }
    }
}

fn state_slot(slots: StateSlots, slot: Word, delta: Word) -> OpResult<&Option<Word>> {
    let delta = bool_from_word(delta).ok_or(AccessError::InvalidStateSlotDelta(delta))?;
    let slots = state_slots_from_delta(slots, delta);
    let ix = usize::try_from(slot).map_err(|_| AccessError::StateSlotOutOfBounds)?;
    let slot = slots.get(ix).ok_or(AccessError::StateSlotOutOfBounds)?;
    Ok(slot)
}

fn state_slot_range(
    slots: StateSlots,
    slot: Word,
    len: Word,
    delta: Word,
) -> OpResult<&StateSlotSlice> {
    let delta = bool_from_word(delta).ok_or(AccessError::InvalidStateSlotDelta(slot))?;
    let slots = state_slots_from_delta(slots, delta);
    let range = range_from_start_len(slot, len).ok_or(AccessError::StateSlotOutOfBounds)?;
    let subslice = slots
        .get(range)
        .ok_or(AccessError::DecisionSlotOutOfBounds)?;
    Ok(subslice)
}

fn range_from_start_len(start: Word, len: Word) -> Option<std::ops::Range<usize>> {
    let start = usize::try_from(start).ok()?;
    let len = usize::try_from(len).ok()?;
    let end = start.checked_add(len)?;
    Some(start..end)
}

fn state_slots_from_delta(slots: StateSlots, delta: bool) -> &StateSlotSlice {
    if delta {
        slots.post
    } else {
        slots.pre
    }
}

#[cfg(test)]
mod tests {
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
                        decision_variables: vec![DecisionVariable::Transient(
                            DecisionVariableIndex {
                                solution_data_index: 1,
                                variable_index: 0,
                            },
                        )],
                    },
                    SolutionData {
                        intent_to_solve: TEST_INTENT_ADDR,
                        decision_variables: vec![DecisionVariable::Transient(
                            DecisionVariableIndex {
                                solution_data_index: 0,
                                variable_index: 0,
                            },
                        )],
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
            Err(ConstraintError::Op(_, OpError::Access(AccessError::InvalidStateSlotDelta(2)))) => {
            }
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
}
