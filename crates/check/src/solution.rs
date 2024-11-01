//! Items related to validating `Solution`s.

use crate::{
    constraint_vm::{
        self,
        error::{CheckError, ConstraintErrors, ConstraintsUnsatisfied},
    },
    state_read_vm::{
        self, asm::FromBytesError, error::StateReadError, Access, BytecodeMapped, Gas, GasLimit,
        SolutionAccess, StateRead, StateSlotSlice, StateSlots,
    },
    types::{
        predicate::Predicate,
        solution::{Solution, SolutionData, SolutionDataIndex},
        Key, PredicateAddress, StateReadBytecode, Word,
    },
};
#[cfg(feature = "tracing")]
use essential_hash::content_addr;
use std::{collections::HashSet, fmt, sync::Arc};
use thiserror::Error;
use tokio::task::JoinSet;
#[cfg(feature = "tracing")]
use tracing::Instrument;

/// Configuration options passed to [`check_predicate`].
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct CheckPredicateConfig {
    /// Whether or not to wait and collect all failures after a single state
    /// read or constraint fails.
    ///
    /// Potentially useful for debugging or testing tools.
    ///
    /// Default: `false`
    pub collect_all_failures: bool,
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
    /// State mutation entry error.
    #[error("Invalid state mutation entry: {0}")]
    StateMutationEntry(KvError),
    /// Decision variable value too large.
    #[error("Decision variable value len {0} exceeds limit {MAX_VALUE_SIZE}")]
    DecVarValueTooLarge(usize),
}

/// Error with a slot key or value.
#[derive(Debug, Error)]
pub enum KvError {
    /// The key is too large.
    #[error("key with length {0} exceeds limit {MAX_KEY_SIZE}")]
    KeyTooLarge(usize),
    /// The value is too large.
    #[error("value with length {0} exceeds limit {MAX_VALUE_SIZE}")]
    ValueTooLarge(usize),
}

/// [`check_state_mutations`] error.
#[derive(Debug, Error)]
pub enum InvalidStateMutations {
    /// The number of state mutations exceeds the limit.
    #[error("the number of state mutations ({0}) exceeds the limit ({MAX_STATE_MUTATIONS})")]
    TooMany(usize),
    /// Discovered multiple mutations to the same slot.
    #[error("attempt to apply multiple mutations to the same slot: {0:?} {1:?}")]
    MultipleMutationsForSlot(PredicateAddress, Key),
}

/// [`check_predicates`] error.
#[derive(Debug, Error)]
pub enum PredicatesError<E> {
    /// One or more solution data failed their associated predicate checks.
    #[error("{0}")]
    Failed(#[from] PredicateErrors<E>),
    /// One or more tasks failed to join.
    #[error("one or more spawned tasks failed to join: {0}")]
    Join(#[from] tokio::task::JoinError),
    /// Summing solution data gas resulted in overflow.
    #[error("summing solution data gas overflowed")]
    GasOverflowed,
}

/// Predicate checking failed for the solution data at the given indices.
#[derive(Debug, Error)]
pub struct PredicateErrors<E>(pub Vec<(SolutionDataIndex, PredicateError<E>)>);

/// [`check_predicate`] error.
#[derive(Debug, Error)]
pub enum PredicateError<E> {
    /// Failed to parse ops from bytecode during bytecode mapping.
    #[error("failed to parse an op during bytecode mapping: {0}")]
    OpsFromBytesError(#[from] FromBytesError),
    /// Failed to read state.
    #[error("state read execution error: {0}")]
    StateRead(#[from] StateReadError<E>),
    /// Constraint checking failed.
    #[error("constraint checking failed: {0}")]
    Constraints(#[from] PredicateConstraintsError),
}

/// The number of decision variables provided by the solution data differs to
/// the number expected by the predicate.
#[derive(Debug, Error)]
#[error("number of solution data decision variables ({data}) differs from predicate ({predicate})")]
pub struct InvalidDecisionVariablesLength {
    /// Number of decision variables provided by solution data.
    pub data: usize,
    /// Number of decision variables expected by the solution data's associated predicate.
    pub predicate: u32,
}

/// [`check_predicate_constraints`] error.
#[derive(Debug, Error)]
pub enum PredicateConstraintsError {
    /// Constraint checking failed.
    #[error("check failed: {0}")]
    Check(#[from] constraint_vm::error::CheckError),
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
/// Maximum number of words in a slot value.
pub const MAX_VALUE_SIZE: usize = 10_000;
/// Maximum number of words in a slot key.
pub const MAX_KEY_SIZE: usize = 1000;

impl<E: fmt::Display> fmt::Display for PredicateErrors<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("predicate checking failed for one or more solution data:\n")?;
        for (ix, err) in &self.0 {
            f.write_str(&format!("  {ix}: {err}\n"))?;
        }
        Ok(())
    }
}

/// Validate a solution, to the extent it can be validated without reference to
/// its associated predicates.
///
/// This includes solution data and state mutations.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(solution = %content_addr(solution)), err))]
pub fn check(solution: &Solution) -> Result<(), InvalidSolution> {
    check_data(&solution.data)?;
    check_state_mutations(solution)?;
    Ok(())
}

fn check_value_size(value: &[Word]) -> Result<(), KvError> {
    if value.len() > MAX_VALUE_SIZE {
        Err(KvError::ValueTooLarge(value.len()))
    } else {
        Ok(())
    }
}

fn check_key_size(value: &[Word]) -> Result<(), KvError> {
    if value.len() > MAX_KEY_SIZE {
        Err(KvError::KeyTooLarge(value.len()))
    } else {
        Ok(())
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

    // Check whether we have too many decision vars
    for (data_ix, data) in data_slice.iter().enumerate() {
        // Ensure the length limit is not exceeded.
        if data.decision_variables.len() > MAX_DECISION_VARIABLES as usize {
            return Err(InvalidSolutionData::TooManyDecisionVariables(
                data_ix,
                data.decision_variables.len(),
            ));
        }
        for v in &data.decision_variables {
            check_value_size(v).map_err(|_| InvalidSolutionData::DecVarValueTooLarge(v.len()))?;
        }
    }
    Ok(())
}

/// Validate the solution's state mutations.
pub fn check_state_mutations(solution: &Solution) -> Result<(), InvalidSolution> {
    // Validate state mutations.
    // Ensure that solution state mutations length is below limit length.
    if solution.state_mutations_len() > MAX_STATE_MUTATIONS {
        return Err(InvalidStateMutations::TooMany(solution.state_mutations_len()).into());
    }

    // Ensure that no more than one mutation per slot is proposed.
    for data in &solution.data {
        let mut mut_keys = HashSet::new();
        for mutation in &data.state_mutations {
            if !mut_keys.insert(&mutation.key) {
                return Err(InvalidStateMutations::MultipleMutationsForSlot(
                    data.predicate_to_solve.clone(),
                    mutation.key.clone(),
                )
                .into());
            }
            // Check key length.
            check_key_size(&mutation.key).map_err(InvalidSolutionData::StateMutationEntry)?;
            // Check value length.
            check_value_size(&mutation.value).map_err(InvalidSolutionData::StateMutationEntry)?;
        }
    }

    Ok(())
}

/// Checks all of a solution's `SolutionData` against its associated predicates.
///
/// For each of the solution's `data` elements, a single task is spawned that
/// reads the pre and post state slots for the associated predicate with access to
/// the given `pre_state` and `post_state`, then checks all constraints over the
/// resulting pre and post state slots.
///
/// **NOTE:** This assumes that the given `Solution` and all `Predicate`s
/// have already been independently validated using
/// [`solution::check`][crate::solution::check] and
/// [`predicate::check`][crate::predicate::check] respectively.
///
/// ## Arguments
///
/// - `pre_state` must provide access to state *prior to* mutations being applied.
/// - `post_state` must provide access to state *post* mutations being applied.
/// - `get_predicate` provides immediate access to a predicate associated with the given
///   solution. Calls to `predicate` must complete immediately. All necessary
///   predicates are assumed to have been read from storage and validated ahead of time.
///
/// Returns the total gas spent.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub async fn check_predicates<SA, SB>(
    pre_state: &SA,
    post_state: &SB,
    solution: Arc<Solution>,
    get_predicate: impl Fn(&PredicateAddress) -> Arc<Predicate>,
    config: Arc<CheckPredicateConfig>,
) -> Result<Gas, PredicatesError<SA::Error>>
where
    SA: Clone + StateRead + Send + Sync + 'static,
    SB: Clone + StateRead<Error = SA::Error> + Send + Sync + 'static,
    SA::Future: Send,
    SB::Future: Send,
    SA::Error: Send,
{
    #[cfg(feature = "tracing")]
    tracing::trace!("{}", essential_hash::content_addr(&*solution));

    // Read pre and post states then check constraints.
    let mut set: JoinSet<(_, Result<_, PredicateError<SA::Error>>)> = JoinSet::new();
    for (solution_data_index, data) in solution.data.iter().enumerate() {
        let solution_data_index: SolutionDataIndex = solution_data_index
            .try_into()
            .expect("solution data index already validated");
        let predicate = get_predicate(&data.predicate_to_solve);
        let solution = solution.clone();
        let pre_state: SA = pre_state.clone();
        let post_state: SB = post_state.clone();
        let config = config.clone();

        let future = async move {
            let pre_state = pre_state;
            let post_state = post_state;
            let res = check_predicate(
                &pre_state,
                &post_state,
                solution,
                predicate,
                solution_data_index,
                &config,
            )
            .await;
            (solution_data_index, res)
        };

        #[cfg(feature = "tracing")]
        let future = future.in_current_span();

        set.spawn(future);
    }

    // Calculate gas used.
    // TODO: Gas is only calculated for state reads.
    // Add gas tracking for constraint checking.
    let mut total_gas: u64 = 0;
    let mut failed = vec![];
    while let Some(res) = set.join_next().await {
        let (solution_data_ix, res) = res?;
        let g = match res {
            Ok(ok) => ok,
            Err(e) => {
                failed.push((solution_data_ix, e));
                if config.collect_all_failures {
                    continue;
                } else {
                    return Err(PredicateErrors(failed).into());
                }
            }
        };

        total_gas = total_gas
            .checked_add(g)
            .ok_or(PredicatesError::GasOverflowed)?;
    }

    // If any predicates failed, return an error.
    if !failed.is_empty() {
        return Err(PredicateErrors(failed).into());
    }

    Ok(total_gas)
}

/// Checks a solution against a single predicate using the solution data at the given index.
///
/// Reads all pre and post state slots into memory, then checks all constraints.
///
/// **NOTE:** This assumes that the given `Solution` and `Predicate` have been
/// independently validated using [`solution::check`][crate::solution::check]
/// and [`predicate::check`][crate::predicate::check] respectively.
///
/// ## Arguments
///
/// - `pre_state` must provide access to state *prior to* mutations being applied.
/// - `post_state` must provide access to state *post* mutations being applied.
/// - `solution_data_index` represents the data within `solution.data` that claims
///   to solve this predicate.
///
/// Returns the total gas spent.
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(
        skip_all,
        fields(
            solution = %format!("{}", content_addr(&*solution))[0..8],
            data={solution_data_index},
        ),
    ),
)]
pub async fn check_predicate<SA, SB>(
    pre_state: &SA,
    post_state: &SB,
    solution: Arc<Solution>,
    predicate: Arc<Predicate>,
    solution_data_index: SolutionDataIndex,
    config: &CheckPredicateConfig,
) -> Result<Gas, PredicateError<SA::Error>>
where
    SA: StateRead,
    SB: StateRead<Error = SA::Error>,
{
    // Perform the state reads and construct the state slots.
    let (state_read_gas, pre_slots, post_slots) = predicate_state_slots(
        pre_state,
        post_state,
        &solution,
        &predicate.state_read,
        solution_data_index,
    )
    .await?;

    // Check constraints.
    check_predicate_constraints(
        solution,
        solution_data_index,
        predicate.clone(),
        Arc::from(pre_slots.into_boxed_slice()),
        Arc::from(post_slots.into_boxed_slice()),
        config,
    )
    .await?;

    Ok(state_read_gas)
}

/// Pre-state slots generated from state reads.
pub type PreStateSlots = Vec<Vec<Word>>;

/// Post-state slots generated from state reads.
pub type PostStateSlots = Vec<Vec<Word>>;

/// Reads all pre and post state slots for the given predicate into memory for
/// checking the solution data at the given index.
///
/// Returns a tuple with the total gas spent, the pre-state slots, and the
/// post-state slots respectively.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub async fn predicate_state_slots<SA, SB>(
    pre_state: &SA,
    post_state: &SB,
    solution: &Solution,
    predicate_state_reads: &[StateReadBytecode],
    solution_data_index: SolutionDataIndex,
) -> Result<(Gas, PreStateSlots, PostStateSlots), PredicateError<SA::Error>>
where
    SA: StateRead,
    SB: StateRead<Error = SA::Error>,
{
    // Track the total gas spent over all execution.
    let mut total_gas = 0;

    // Initialize pre and post slots. These will contain all state slots for all state reads.
    let mut pre_slots: Vec<Vec<Word>> = Vec::new();
    let mut post_slots: Vec<Vec<Word>> = Vec::new();
    let mutable_keys = constraint_vm::mut_keys_set(solution, solution_data_index);
    let solution_access = SolutionAccess::new(solution, solution_data_index, &mutable_keys);

    // Read pre and post states.
    for (state_read_index, state_read) in predicate_state_reads.iter().enumerate() {
        #[cfg(not(feature = "tracing"))]
        let _ = state_read_index;

        // Map the bytecode ops ahead of execution to share the mapping
        // between both pre and post state slot reads.
        let state_read_mapped = BytecodeMapped::try_from(&state_read[..])?;

        // Read pre state slots and write them to the pre_slots slice.
        let future = read_state_slots(
            &state_read_mapped,
            Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            },
            pre_state,
        );
        #[cfg(feature = "tracing")]
        let (gas, new_pre_slots) = future
            .instrument(tracing::info_span!("pre", ix = state_read_index))
            .await?;
        #[cfg(not(feature = "tracing"))]
        let (gas, new_pre_slots) = future.await?;

        total_gas += gas;
        pre_slots.extend(new_pre_slots);

        // Read post state slots and write them to the post_slots slice.
        let future = read_state_slots(
            &state_read_mapped,
            Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            },
            post_state,
        );
        #[cfg(feature = "tracing")]
        let (gas, new_post_slots) = future
            .instrument(tracing::info_span!("post", ix = state_read_index))
            .await?;
        #[cfg(not(feature = "tracing"))]
        let (gas, new_post_slots) = future.await?;

        total_gas += gas;
        post_slots.extend(new_post_slots);
    }

    Ok((total_gas, pre_slots, post_slots))
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
) -> Result<(Gas, Vec<Vec<Word>>), state_read_vm::error::StateReadError<S::Error>>
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

    Ok((gas_spent, vm.into_state_slots()))
}

/// Checks if the given solution data at the given index satisfies the
/// constraints of the given predicate.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, "check"))]
pub async fn check_predicate_constraints(
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
    predicate: Arc<Predicate>,
    pre_slots: Arc<StateSlotSlice>,
    post_slots: Arc<StateSlotSlice>,
    config: &CheckPredicateConfig,
) -> Result<(), PredicateConstraintsError> {
    let r = check_predicate_constraints_parallel(
        solution.clone(),
        solution_data_index,
        predicate.clone(),
        pre_slots.clone(),
        post_slots.clone(),
        config,
    )
    .await;
    #[cfg(feature = "tracing")]
    if let Err(ref err) = r {
        tracing::trace!("error checking constraints: {}", err);
    }
    r
}

/// Check predicates in parallel without sleeping any threads.
async fn check_predicate_constraints_parallel(
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
    predicate: Arc<Predicate>,
    pre_slots: Arc<StateSlotSlice>,
    post_slots: Arc<StateSlotSlice>,
    config: &CheckPredicateConfig,
) -> Result<(), PredicateConstraintsError> {
    let mut handles = Vec::with_capacity(predicate.constraints.len());

    // Spawn each constraint onto a rayon thread and
    // check them in parallel.
    for ix in 0..predicate.constraints.len() {
        // Spawn this sync code onto a rayon thread.
        // This is a non-blocking operation.
        let (tx, rx) = tokio::sync::oneshot::channel();
        handles.push(rx);

        // These are all cheap Arc clones.
        let solution = solution.clone();
        let pre_slots = pre_slots.clone();
        let post_slots = post_slots.clone();
        let predicate = predicate.clone();

        #[cfg(feature = "tracing")]
        let span = tracing::Span::current();

        rayon::spawn(move || {
            #[cfg(feature = "tracing")]
            let span = tracing::trace_span!(parent: &span, "constraint", ix = ix as u32);
            #[cfg(feature = "tracing")]
            let guard = span.enter();

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
                predicate
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

            #[cfg(feature = "tracing")]
            drop(guard)
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
