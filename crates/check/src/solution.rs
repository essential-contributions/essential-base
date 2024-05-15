//! Items related to validating `Solution`s.

use crate::{
    constraint_vm::{
        self,
        error::{CheckError, ConstraintErrors, ConstraintsUnsatisfied},
    },
    sign::{self, secp256k1},
    state_read_vm::{
        self, asm::FromBytesError, error::StateReadError, Access, BytecodeMapped, Gas, GasLimit,
        SolutionAccess, StateRead, StateSlotSlice, StateSlots,
    },
    types::{
        intent::{Directive, Intent},
        slots::{self, StateSlot},
        solution::{
            DecisionVariable, DecisionVariableIndex, Solution, SolutionData, SolutionDataIndex,
        },
        IntentAddress, Key, Signed, Word,
    },
};
use std::{collections::HashSet, fmt, sync::Arc};
use thiserror::Error;
use tokio::task::JoinSet;
#[cfg(feature = "tracing")]
use tracing_futures::Instrument;

/// Configuration options passed to [`check_intent`].
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct CheckIntentConfig {
    /// Whether or not to wait and collect all failures after a single state
    /// read or constraint fails.
    ///
    /// Potentially useful for debugging or testing tools.
    ///
    /// Default: `false`
    pub collect_all_failures: bool,
}

/// [`check_signed`] error.
#[derive(Debug, Error)]
pub enum InvalidSignedSolution {
    /// Invalid signature.
    #[error("failed to validate solution signature")]
    Signature(#[from] secp256k1::Error),
    /// Invalid solution.
    #[error("invalid solution: {0}")]
    Solution(#[from] InvalidSolution),
}

/// [`check`] error.
#[derive(Debug, Error)]
pub enum InvalidSolution {
    /// Invalid solution data.
    #[error("invalid solution data: {0}")]
    Data(#[from] InvalidSolutionData),
    /// State mutations validation failed.
    #[error("state mutations validation failed: {0}")]
    StateMutations(#[from] InvalidStateMutations),
}

/// [`check_data`] error.
#[derive(Debug, Error)]
pub enum InvalidSolutionData {
    /// There must be at least one solution data.
    #[error("must be at least one solution data")]
    Empty,
    /// The number of solution data exceeds the limit.
    #[error("the number of solution data ({0}) exceeds the limit ({MAX_SOLUTION_DATA})")]
    TooMany(usize),
    /// A solution data expects too many decision variables.
    #[error("data {0} expects too many decision vars {1} (limit: {MAX_DECISION_VARIABLES})")]
    TooManyDecisionVariables(usize, usize),
    /// A decision variable fails to resolve within the solution's data.
    #[error("the following decision variable fails to resolve: {0:?}")]
    UnresolvingDecisionVariable(DecisionVariableIndex),
    /// A set of decision variables were found to cause a cycle during resolution.
    #[error("the following set of decision variables form a cycle: {0:?}")]
    DecisionVariablesCycle(HashSet<DecisionVariableIndex>),
}

/// [`check_state_mutations`] error.
#[derive(Debug, Error)]
pub enum InvalidStateMutations {
    /// The number of state mutations exceeds the limit.
    #[error("the number of state mutations ({0}) exceeds the limit ({MAX_STATE_MUTATIONS})")]
    TooMany(usize),
    /// State mutation pathway at the given index is out of range of solution data.
    #[error("state mutation pathway {0} out of range of solution data")]
    PathwayOutOfRangeOfSolutionData(u16),
    /// Discovered multiple mutations to the same slot.
    #[error("attempt to apply multiple mutations to the same slot: {0:?} {1:?}")]
    MultipleMutationsForSlot(IntentAddress, Key),
}

/// [`check_intents`] error.
#[derive(Debug, Error)]
pub enum IntentsError<E> {
    /// One or more solution data failed their associated intent checks.
    #[error("{0}")]
    Failed(#[from] IntentErrors<E>),
    /// One or more tasks failed to join.
    #[error("one or more spawned tasks failed to join: {0}")]
    Join(#[from] tokio::task::JoinError),
    /// Summing solution data utility resulted in overflow.
    #[error("summing solution data utility overflowed")]
    UtilityOverflowed,
    /// Summing solution data gas resulted in overflow.
    #[error("summing solution data gas overflowed")]
    GasOverflowed,
}

/// Intent checking failed for the solution data at the given indices.
#[derive(Debug, Error)]
pub struct IntentErrors<E>(pub Vec<(SolutionDataIndex, IntentError<E>)>);

/// [`check_intent`] error.
#[derive(Debug, Error)]
pub enum IntentError<E> {
    /// The number of decision variables provided by the solution data differs
    /// from the number expected by the intent.
    #[error("{0}")]
    DecisionVariablesMismatch(#[from] InvalidDecisionVariablesLength),
    /// Failed to parse ops from bytecode during bytecode mapping.
    #[error("failed to parse an op during bytecode mapping: {0}")]
    OpsFromBytesError(#[from] FromBytesError),
    /// Failed to read state.
    #[error("state read execution error: {0}")]
    StateRead(#[from] StateReadError<E>),
    /// Failed to write state slots to temporary slice.
    #[error("failed to write state slots to temporary slice: {0}")]
    WriteStateSlots(#[from] WriteStateSlotsError),
    /// Constraint checking failed.
    #[error("constraint checking failed: {0}")]
    Constraints(#[from] IntentConstraintsError),
}

/// The number of decision variables provided by the solution data differs to
/// the number expected by the intent.
#[derive(Debug, Error)]
#[error("number of solution data decision variables ({data}) differs from intent ({intent})")]
pub struct InvalidDecisionVariablesLength {
    /// Number of decision variables provided by solution data.
    pub data: usize,
    /// Number of decision variables expected by the solution data's associated intent.
    pub intent: u32,
}

/// Failed to write state slots.
#[derive(Debug, Error)]
pub enum WriteStateSlotsError {
    /// No program index matching state read index.
    #[error("no program index matching state read index {0}")]
    NoProgramIndexMatchingStateReadIndex(u16),
    /// Length of read state slots does not match expected length.
    #[error("length of read state slots ({found}) does not match expected length ({expected})")]
    StateSlotLengthMismatch {
        /// The number of read state slots.
        found: usize,
        /// The expected number of state slots.
        expected: usize,
    },
}

/// [`check_intent_constraints`] error.
#[derive(Debug, Error)]
pub enum IntentConstraintsError {
    /// Constraint checking failed.
    #[error("check failed: {0}")]
    Check(#[from] constraint_vm::error::CheckError),
    /// Failed to receive result from spawned task.
    #[error("failed to recv: {0}")]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),
    /// Failed to calculate the utility.
    #[error("failed to calculate utility: {0}")]
    Utility(#[from] UtilityError),
}

/// The utility score of a solution.
pub type Utility = f64;

/// `calculate_utility` error.
#[derive(Debug, Error)]
pub enum UtilityError {
    /// The range specified by the intent's directive is invalid.
    #[error("the range specified by the directive [{0}..{1}] is invalid")]
    InvalidDirectiveRange(Word, Word),
    /// The stack returned from directive execution is invalid.
    #[error("invalid stack result after directive execution: {0}")]
    InvalidStack(#[from] constraint_vm::error::StackError),
    /// Failed to execute the directive using the constraint VM.
    #[error("directive execution with constraint VM failed: {0}")]
    Execution(#[from] constraint_vm::error::ConstraintError),
    /// Failed to receive result from spawned task.
    #[error("failed to recv: {0}")]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),
}

/// Maximum number of decision variables of a solution.
pub const MAX_DECISION_VARIABLES: u32 = 100;
/// Maximum number of solution data of a solution.
pub const MAX_SOLUTION_DATA: usize = 100;
/// Maximum number of state mutations of a solution.
pub const MAX_STATE_MUTATIONS: usize = 1000;

impl<E: fmt::Display> fmt::Display for IntentErrors<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("intent checking failed for one or more solution data:\n")?;
        for (ix, err) in &self.0 {
            f.write_str(&format!("  {ix}: {err}\n"))?;
        }
        Ok(())
    }
}

/// Validate a [`Signed<Solution>`][Signed], to the extent it can be validated
/// without reference to its associated intents.
///
/// This includes solution data and state mutations.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub fn check_signed(solution: &Signed<Solution>) -> Result<(), InvalidSignedSolution> {
    match sign::verify(solution) {
        Ok(()) => {
            check(&solution.data)?;
            Ok(())
        }
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "error verifying signature of solution with hash 0x{}: {}",
                hex::encode(essential_hash::hash(&solution.data)),
                err
            );
            Err(err.into())
        }
    }
}

/// Validate a solution, to the extent it can be validated without reference to
/// its associated intents.
///
/// This includes solution data and state mutations.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub fn check(solution: &Solution) -> Result<(), InvalidSolution> {
    match check_data(&solution.data) {
        Ok(()) => match check_state_mutations(solution) {
            Ok(()) => Ok(()),
            Err(err) => {
                #[cfg(feature = "tracing")]
                tracing::debug!(
                    "invalid state mutations for solution with hash 0x{}: {}",
                    hex::encode(essential_hash::hash(&solution.data)),
                    err
                );
                Err(err.into())
            }
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "invalid data for solution with hash 0x{}: {}",
                hex::encode(essential_hash::hash(&solution.data)),
                err
            );
            Err(err.into())
        }
    }
}

/// Validate the solution's slice of [`SolutionData`].
pub fn check_data(data_slice: &[SolutionData]) -> Result<(), InvalidSolutionData> {
    // Validate solution data.
    // Ensure that at solution has at least one solution data.
    if data_slice.is_empty() {
        return Err(InvalidSolutionData::Empty);
    }
    // Ensure that solution data length is below limit length.
    if data_slice.len() > MAX_SOLUTION_DATA {
        return Err(InvalidSolutionData::TooMany(data_slice.len()));
    }

    // Check whether we have too many decision vars, or if any don't resolve.
    // We track already `resolved` variables to avoid checking them multiple times.
    // We re-use a `visited` set between transient entry points to check for cycles.
    let mut resolved = HashSet::new();
    let mut visited = HashSet::new(); // Re-used to track visited dec vars, checking for cycles.
    for (data_ix, data) in data_slice.iter().enumerate() {
        // Ensure the length limit is not exceeded.
        if data.decision_variables.len() > MAX_DECISION_VARIABLES as usize {
            return Err(InvalidSolutionData::TooManyDecisionVariables(
                data_ix,
                data.decision_variables.len(),
            ));
        }

        // Ensure that all transient decision variables resolve without cycling.
        for var_ix in 0..data.decision_variables.len() {
            let mut ix = DecisionVariableIndex {
                solution_data_index: u16::try_from(data_ix).expect("checked prev"),
                variable_index: u16::try_from(var_ix).expect("checked prev"),
            };

            // If we already know this resolves because it was previously
            // visited in a successful resolution, we can skip the following check.
            if resolved.contains(&ix) {
                continue;
            }

            // Reset our visited set.
            visited.clear();
            loop {
                let dec_var = data_slice
                    .get(ix.solution_data_index as usize)
                    .and_then(|data| data.decision_variables.get(ix.variable_index as usize))
                    .ok_or(InvalidSolutionData::UnresolvingDecisionVariable(ix))?;

                // We managed to resolve both data and the dec var for this index.
                let already_resolved = !resolved.insert(ix);

                match *dec_var {
                    DecisionVariable::Inline(_w) => break,
                    DecisionVariable::Transient(ref transient) => {
                        // We're traversing transient data, so track vars already visited.
                        if !visited.insert(ix) {
                            return Err(InvalidSolutionData::DecisionVariablesCycle(visited));
                        }
                        // Now that we know we're not cycling and this transient
                        // var has already been resolved before, we're done.
                        if already_resolved {
                            break;
                        }
                        // Otherwise, continue resolving.
                        ix = *transient;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Validate the solution's state mutations.
pub fn check_state_mutations(solution: &Solution) -> Result<(), InvalidStateMutations> {
    // Validate state mutations.
    // Ensure that solution state mutations length is below limit length.
    if solution.state_mutations.len() > MAX_STATE_MUTATIONS {
        return Err(InvalidStateMutations::TooMany(
            solution.state_mutations.len(),
        ));
    }

    // Ensure that all state mutations with a pathway points to some solution data.
    for state_mut in &solution.state_mutations {
        if solution.data.len() <= usize::from(state_mut.pathway) {
            return Err(InvalidStateMutations::PathwayOutOfRangeOfSolutionData(
                state_mut.pathway,
            ));
        }
    }

    // Ensure that no more than one mutation per slot is proposed.
    let mut mut_keys = HashSet::new();
    for state_mutation in &solution.state_mutations {
        let intent_addr = &solution.data[state_mutation.pathway as usize].intent_to_solve;
        for mutation in &state_mutation.mutations {
            if !mut_keys.insert((intent_addr, &mutation.key)) {
                return Err(InvalidStateMutations::MultipleMutationsForSlot(
                    intent_addr.clone(),
                    mutation.key,
                ));
            }
        }
    }

    Ok(())
}

/// Checks all of a solution's `SolutionData` against its associated intents.
///
/// For each of the solution's `data` elements, a single task is spawned that
/// reads the pre and post state slots for the associated intent with access to
/// the given `pre_state` and `post_state`, then checks all constraints over the
/// resulting pre and post state slots.
///
/// **NOTE:** This assumes that the given `Solution` and all `Intent`s
/// have already been independently validated using
/// [`solution::check`][crate::solution::check] and
/// [`intent::check`][crate::intent::check] respectively.
///
/// ## Arguments
///
/// - `pre_state` must provide access to state *prior to* mutations being applied.
/// - `post_state` must provide access to state *post* mutations being applied.
/// - `get_intent` provides immediate access to an intent associated with the given
///   solution. Calls to `intent` must complete immediately. All necessary
///   intents are assumed to have been read from storage and validated ahead of time.
///
/// Returns the utility score of the solution alongside the total gas spent.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub async fn check_intents<SA, SB>(
    pre_state: &SA,
    post_state: &SB,
    solution: Arc<Solution>,
    get_intent: impl Fn(&IntentAddress) -> Arc<Intent>,
    config: Arc<CheckIntentConfig>,
) -> Result<(Utility, Gas), IntentsError<SA::Error>>
where
    SA: Clone + StateRead + Send + Sync + 'static,
    SB: Clone + StateRead<Error = SA::Error> + Send + Sync + 'static,
    SA::Future: Send,
    SB::Future: Send,
    SA::Error: Send,
{
    // Check decision variable lengths before spawning tasks.
    if let Err((ix, err)) = check_decision_variable_lengths(&solution, &get_intent) {
        let failed = vec![(ix, IntentError::DecisionVariablesMismatch(err))];
        return Err(IntentErrors(failed).into());
    }

    #[cfg(feature = "tracing")]
    tracing::trace!("{:?}", &solution);

    // Read pre and post states then check constraints.
    let mut set: JoinSet<(_, Result<_, IntentError<SA::Error>>)> = JoinSet::new();
    for (solution_data_index, data) in solution.data.iter().enumerate() {
        let solution_data_index: SolutionDataIndex = solution_data_index
            .try_into()
            .expect("solution data index already validated");
        let intent = get_intent(&data.intent_to_solve);

        #[cfg(feature = "tracing")]
        tracing::trace!(
            "solution_data_index: {}\n{:?}",
            solution_data_index,
            &intent
        );

        let solution = solution.clone();
        let pre_state: SA = pre_state.clone();
        let post_state: SB = post_state.clone();
        let config = config.clone();

        let future = async move {
            let pre_state = pre_state;
            let post_state = post_state;
            let res = check_intent(
                &pre_state,
                &post_state,
                solution,
                intent,
                solution_data_index,
                &config,
            )
            .await;
            (solution_data_index, res)
        };
        #[cfg(feature = "tracing")]
        set.spawn(future.instrument(tracing::info_span!("check_intent")));
        #[cfg(not(feature = "tracing"))]
        set.spawn(future);
    }

    // Calculate total utility and gas used.
    // TODO: Gas is only calculated for state reads.
    // Add gas tracking for constraint checking.
    let mut total_gas: u64 = 0;
    let mut utility: f64 = 0.0;
    let mut failed = vec![];
    while let Some(res) = set.join_next().await {
        let (solution_data_ix, res) = res?;
        let (u, g) = match res {
            Ok(ok) => ok,
            Err(e) => {
                failed.push((solution_data_ix, e));
                if config.collect_all_failures {
                    continue;
                } else {
                    return Err(IntentErrors(failed).into());
                }
            }
        };
        utility += u;

        if utility == f64::INFINITY {
            return Err(IntentsError::UtilityOverflowed);
        }

        total_gas = total_gas
            .checked_add(g)
            .ok_or(IntentsError::GasOverflowed)?;
    }

    // If any intents failed, return an error.
    if !failed.is_empty() {
        return Err(IntentErrors(failed).into());
    }

    Ok((utility, total_gas))
}

/// Validate the solution data decision variables against those expected by their associated intent.
///
/// This function assumes that `Solution` and `Intent` have already been
/// independently validated, and may `panic!` otherwise.
///
/// Upon error, returns the index of the failed data alongside the error.
pub fn check_decision_variable_lengths(
    solution: &Solution,
    get_intent: impl Fn(&IntentAddress) -> Arc<Intent>,
) -> Result<(), (u16, InvalidDecisionVariablesLength)> {
    for (ix, data) in solution.data.iter().enumerate() {
        let intent = get_intent(&data.intent_to_solve);
        // Ensure the numbers match.
        if data.decision_variables.len() != intent.slots.decision_variables as usize {
            let err = InvalidDecisionVariablesLength {
                data: data.decision_variables.len(),
                intent: intent.slots.decision_variables,
            };
            let ix = u16::try_from(ix).expect("solution data length already validated");
            return Err((ix, err));
        }
    }
    Ok(())
}

/// Checks a solution against a single intent using the solution data at the given index.
///
/// Reads all pre and post state slots into memory, then checks all constraints.
///
/// **NOTE:** This assumes that the given `Solution` and `Intent` have been
/// independently validated using [`solution::check`][crate::solution::check]
/// and [`intent::check`][crate::intent::check] respectively.
///
/// ## Arguments
///
/// - `pre_state` must provide access to state *prior to* mutations being applied.
/// - `post_state` must provide access to state *post* mutations being applied.
/// - `solution_data_index` represents the data within `solution.data` that claims
///   to solve this intent.
///
/// Returns the utility score of the solution alongside the total gas spent.
pub async fn check_intent<SA, SB>(
    pre_state: &SA,
    post_state: &SB,
    solution: Arc<Solution>,
    intent: Arc<Intent>,
    solution_data_index: SolutionDataIndex,
    config: &CheckIntentConfig,
) -> Result<(Utility, Gas), IntentError<SA::Error>>
where
    SA: StateRead + Sync,
    SB: StateRead<Error = SA::Error> + Sync,
{
    // Get the length of state slots for this intent.
    let intent_state_len: usize = slots::state_len(&intent.slots.state)
        .expect("intent state slot length must be validated prior to calling `check_intent`")
        .try_into()
        .expect("`u32` to `usize` conversion cannot fail on 32 or 64-bit machines");

    // Track the total gas spent over all execution.
    let mut total_gas = 0;

    // Initialize pre and post slots. These will contain all state slots for all state reads.
    let mut pre_slots: Vec<Option<Word>> = vec![None; intent_state_len];
    let mut post_slots: Vec<Option<Word>> = vec![None; intent_state_len];
    let mutable_keys = constraint_vm::mut_keys_set(&solution, solution_data_index);
    let solution_access = SolutionAccess::new(&solution, solution_data_index, &mutable_keys);

    // Read pre and post states.
    for (state_read_index, state_read) in intent.state_read.iter().enumerate() {
        let state_read_index: u16 = state_read_index
            .try_into()
            .expect("intent state read count checked previously");

        // Map the bytecode ops ahead of execution to share the mapping
        // between both pre and post state slot reads.
        let state_read_mapped = BytecodeMapped::try_from(&state_read[..])?;

        // Read pre state slots and write them to the pre_slots slice.
        #[cfg(feature = "tracing")]
        tracing::trace!(
            "reading pre-slots for solution_data_index: {} state_read_index: {}",
            solution_data_index,
            state_read_index
        );
        let (gas, new_pre_slots) = read_state_slots(
            &state_read_mapped,
            Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            },
            pre_state,
        )
        .await?;
        total_gas += gas;
        write_state_slots(
            state_read_index,
            &intent.slots.state,
            &mut pre_slots,
            &new_pre_slots,
        )?;

        // Read post state slots and write them to the post_slots slice.
        #[cfg(feature = "tracing")]
        tracing::trace!(
            "reading post-slots for solution_data_index: {} state_read_index: {}",
            solution_data_index,
            state_read_index
        );
        let (gas, new_post_slots) = read_state_slots(
            &state_read_mapped,
            Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            },
            post_state,
        )
        .await?;
        total_gas += gas;
        write_state_slots(
            state_read_index,
            &intent.slots.state,
            &mut post_slots,
            &new_post_slots,
        )?;
    }

    // Check constraints.
    let utility = check_intent_constraints(
        solution,
        solution_data_index,
        intent.clone(),
        Arc::from(pre_slots.into_boxed_slice()),
        Arc::from(post_slots.into_boxed_slice()),
        config,
    )
    .await?;

    Ok((utility, total_gas))
}

/// Reads state slots from storage using the given bytecode.
///
/// The result is written to VM's memory.
///
/// Returns the gas spent alongside the state slots consumed from the VM's memory.
async fn read_state_slots<S>(
    bytecode_mapped: &BytecodeMapped<&[u8]>,
    access: Access<'_>,
    state_read: &S,
) -> Result<(Gas, Box<StateSlotSlice>), state_read_vm::error::StateReadError<S::Error>>
where
    S: StateRead,
{
    // Create a new state read VM.
    let mut vm = state_read_vm::Vm::default();

    // Read the state into the VM's memory.
    let gas_spent = vm
        .exec_bytecode(
            bytecode_mapped,
            access,
            state_read,
            &|_: &state_read_vm::asm::Op| 1,
            GasLimit::UNLIMITED,
        )
        .await?;

    Ok((gas_spent, vm.into_state_slots().into_boxed_slice()))
}

/// Write to the correct slots based on the state read index.
fn write_state_slots(
    state_read_index: u16,
    state_slots: &[StateSlot],
    slots: &mut StateSlotSlice,
    output_slots: &StateSlotSlice,
) -> Result<(), WriteStateSlotsError> {
    // Find the state slot by matching the state read index with the program index.
    let Some(slots) = state_slots
        .iter()
        .find(|slot| slot.program_index == state_read_index)
        .and_then(|slot| {
            let start: usize = slot.index.try_into().ok()?;
            let end: usize = slot.amount.try_into().ok()?;
            let end = end.checked_add(start)?;
            slots.get_mut(start..end)
        })
    else {
        return Err(WriteStateSlotsError::NoProgramIndexMatchingStateReadIndex(
            state_read_index,
        ));
    };

    // The length of the output slots must match the length of the slots
    // that are being written to.
    if slots.len() != output_slots.len() {
        return Err(WriteStateSlotsError::StateSlotLengthMismatch {
            found: output_slots.len(),
            expected: slots.len(),
        });
    }

    // Write the output slots to the correct position in the slots.
    slots.copy_from_slice(output_slots);

    Ok(())
}

/// Checks if the given solution data at the given index satisfies the
/// constraints of the given intent.
///
/// Returns the utility of the solution for the given intent.
pub async fn check_intent_constraints(
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
    intent: Arc<Intent>,
    pre_slots: Arc<StateSlotSlice>,
    post_slots: Arc<StateSlotSlice>,
    config: &CheckIntentConfig,
) -> Result<Utility, IntentConstraintsError> {
    let future = check_intent_constraints_parallel(
        solution.clone(),
        solution_data_index,
        intent.clone(),
        pre_slots.clone(),
        post_slots.clone(),
        config,
    );
    #[cfg(feature = "tracing")]
    future
        .instrument(tracing::info_span!("check_constraints"))
        .await?;
    #[cfg(not(feature = "tracing"))]
    future.await?;
    let util = calculate_utility(
        solution,
        solution_data_index,
        intent.clone(),
        pre_slots,
        post_slots,
    )
    .await?;
    Ok(util)
}

/// Check intents in parallel without sleeping any threads.
async fn check_intent_constraints_parallel(
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
    intent: Arc<Intent>,
    pre_slots: Arc<StateSlotSlice>,
    post_slots: Arc<StateSlotSlice>,
    config: &CheckIntentConfig,
) -> Result<(), IntentConstraintsError> {
    let mut handles = Vec::with_capacity(intent.constraints.len());

    // Spawn each constraint onto a rayon thread and
    // check them in parallel.
    for ix in 0..intent.constraints.len() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        handles.push(rx);

        // These are all cheap Arc clones.
        let solution = solution.clone();
        let pre_slots = pre_slots.clone();
        let post_slots = post_slots.clone();
        let intent = intent.clone();

        // Spawn this sync code onto a rayon thread.
        // This is a non-blocking operation.
        rayon::spawn(move || {
            let mutable_keys = constraint_vm::mut_keys_set(&solution, solution_data_index);
            let solution_access =
                SolutionAccess::new(&solution, solution_data_index, &mutable_keys);
            let access = Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            };
            let res = constraint_vm::eval_bytecode_iter(
                intent
                    .constraints
                    .get(ix)
                    .expect("Safe due to above len check")
                    .iter()
                    .copied(),
                access,
            );
            // Send the result back to the main thread.
            // Send errors are ignored as if the recv is gone there's no one to send to.
            let _ = tx.send((ix, res));
        })
    }

    // There's no way to know the size of these.
    let mut failed = Vec::new();
    let mut unsatisfied = Vec::new();

    // Wait for all constraints to finish.
    // The order of waiting on handles is not important as all
    // constraints make progress independently.
    for handle in handles {
        // Get the index and result from the handle.
        let (ix, res): (usize, Result<bool, _>) = handle.await?;
        match res {
            // If the constraint failed, add it to the failed list.
            Err(err) => {
                failed.push((ix, err));
                if !config.collect_all_failures {
                    break;
                }
            }
            // If the constraint was unsatisfied, add it to the unsatisfied list.
            Ok(b) if !b => unsatisfied.push(ix),
            // Otherwise, the constraint was satisfied.
            _ => (),
        }
    }

    // If there are any failed constraints, return an error.
    if !failed.is_empty() {
        return Err(CheckError::from(ConstraintErrors(failed)).into());
    }

    // If there are any unsatisfied constraints, return an error.
    if !unsatisfied.is_empty() {
        return Err(CheckError::from(ConstraintsUnsatisfied(unsatisfied)).into());
    }
    Ok(())
}

/// Calculates utility of solution for intent.
///
/// Returns utility.
async fn calculate_utility(
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
    intent: Arc<Intent>,
    pre_slots: Arc<StateSlotSlice>,
    post_slots: Arc<StateSlotSlice>,
) -> Result<Utility, UtilityError> {
    match &intent.directive {
        Directive::Satisfy => return Ok(1.0),
        Directive::Maximize(_) | Directive::Minimize(_) => (),
    }

    // Spawn this sync code onto a rayon thread.
    let (tx, rx) = tokio::sync::oneshot::channel();
    rayon::spawn(move || {
        let mutable_keys = constraint_vm::mut_keys_set(&solution, solution_data_index);
        let solution_access = SolutionAccess::new(&solution, solution_data_index, &mutable_keys);
        let access = Access {
            solution: solution_access,
            state_slots: StateSlots {
                pre: &pre_slots,
                post: &post_slots,
            },
        };
        // Extract the directive code.
        let code = match intent.directive {
            Directive::Maximize(ref code) | Directive::Minimize(ref code) => code,
            _ => unreachable!("As this is already checked above"),
        };

        // Execute the directive code.
        let res = constraint_vm::exec_bytecode_iter(code.iter().copied(), access)
            .map_err(UtilityError::from)
            .and_then(|mut stack| {
                let [start, end, value] = stack.pop3()?;
                let util = normalize_utility(value, start, end)?;
                Ok(util)
            });

        // Send errors are ignored as if the recv is dropped.
        let _ = tx.send(res);
    });

    // Await the result of the utility calculation.
    rx.await?
}

fn normalize_utility(value: Word, start: Word, end: Word) -> Result<Utility, UtilityError> {
    if start >= end {
        return Err(UtilityError::InvalidDirectiveRange(start, end));
    }
    let normalized = (value - start) as f64 / (end - start) as f64;
    Ok(normalized.clamp(0.0, 1.0))
}
