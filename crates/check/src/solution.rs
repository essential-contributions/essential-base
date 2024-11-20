//! Items related to validating `Solution`s.

use crate::{
    types::{
        predicate::Predicate,
        solution::{Solution, SolutionData, SolutionDataIndex},
        Key, PredicateAddress, Word,
    },
    vm::{
        self,
        asm::{self, FromBytesError},
        error::{ConstraintsUnsatisfied, MemoryError, StackError, ExecutionError},
        Access, BytecodeMapped, Gas, GasLimit, Memory, Stack, StateRead,
    },
};
#[cfg(feature = "tracing")]
use essential_hash::content_addr;
use essential_types::{
    predicate::{Program, Reads},
    ContentAddress,
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
};
use thiserror::Error;
use tokio::{sync::oneshot, task::JoinSet};
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

/// Required impl for retrieving access to any [`SolutionData`]'s [`Predicate`]s during check.
pub trait GetPredicate {
    /// Provides immediate access to the predicate with the given content address.
    ///
    /// This is called by [`check_predicates`] for each predicate in each solution data being
    /// checked.
    ///
    /// All necessary programs are assumed to have been read from storage and
    /// validated ahead of time.
    fn get_predicate(&self, addr: &PredicateAddress) -> Arc<Predicate>;
}

/// Required impl for retrieving access to any [`Predicate`]'s [`Program`]s during check.
pub trait GetProgram {
    /// Provides immediate access to the program with the given content address.
    ///
    /// This is called by [`check_predicates`] for each node within each predicate for
    /// each solution data being checked.
    ///
    /// All necessary programs are assumed to have been read from storage and
    /// validated ahead of time.
    fn get_program(&self, ca: &ContentAddress) -> Arc<Program>;
}

/// The node context in which a `Program` is evaluated (see [`run_program`]).
struct ProgramCtx {
    /// Oneshot channels providing the result of parent node program evaluation.
    ///
    /// Results in the `Vec` are assumed to be in order of the adjacency list.
    parents: Vec<oneshot::Receiver<Arc<(Stack, Memory)>>>,
    children: Vec<oneshot::Sender<Arc<(Stack, Memory)>>>,
    reads: Reads,
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
    /// One or more program tasks failed to join.
    #[error("one or more spawned program tasks failed to join: {0}")]
    Join(#[from] tokio::task::JoinError),
    /// Failed to retrieve edges for a node, indicating that the predicate's graph is invalid.
    #[error("failed to retrieve edges for node {0} indicating an invalid graph")]
    InvalidNodeEdges(usize),
    /// The execution of one or more programs failed.
    #[error("one or more program execution errors occurred: {0}")]
    ProgramErrors(#[from] ProgramErrors<E>),
    /// One or more of the constraints were unsatisfied.
    #[error("one or more constraints unsatisfied: {0}")]
    ConstraintsUnsatisfied(#[from] ConstraintsUnsatisfied),
}

/// Program execution failed for the programs at the given node indices.
#[derive(Debug, Error)]
pub struct ProgramErrors<E>(Vec<(usize, ProgramError<E>)>);

/// An error occurring during a program task.
#[derive(Debug, Error)]
pub enum ProgramError<E> {
    /// Failed to parse ops from bytecode during bytecode mapping.
    #[error("failed to parse an op during bytecode mapping: {0}")]
    OpsFromBytesError(#[from] FromBytesError),
    /// One of the channels providing a parent program result was dropped.
    #[error("parent result oneshot channel closed: {0}")]
    ParentChannelDropped(#[from] oneshot::error::RecvError),
    /// Concatenating the parent program [`Stack`]s caused an overflow.
    #[error("concatenating parent program `Stack`s caused an overflow: {0}")]
    ParentStackConcatOverflow(#[from] StackError),
    /// Concatenating the parent program [`Memory`] slices caused an overflow.
    #[error("concatenating parent program `Memory` slices caused an overflow: {0}")]
    ParentMemoryConcatOverflow(#[from] MemoryError),
    /// VM execution resulted in an error.
    #[error("VM execution error: {0}")]
    Vm(#[from] ExecutionError<E>),
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
    Check(#[from] vm::error::CheckError),
    /// Failed to receive result from spawned task.
    #[error("failed to recv: {0}")]
    Recv(#[from] oneshot::error::RecvError),
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

impl<E: fmt::Display> fmt::Display for ProgramErrors<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("the programs at the following node indices failed: \n")?;
        for (node_ix, err) in &self.0 {
            f.write_str(&format!("  {node_ix}: {err}\n"))?;
        }
        Ok(())
    }
}

impl<F> GetPredicate for F
where
    F: Fn(&PredicateAddress) -> Arc<Predicate>,
{
    fn get_predicate(&self, addr: &PredicateAddress) -> Arc<Predicate> {
        (*self)(addr)
    }
}

impl<F> GetProgram for F
where
    F: Fn(&ContentAddress) -> Arc<Program>,
{
    fn get_program(&self, ca: &ContentAddress) -> Arc<Program> {
        (*self)(ca)
    }
}

impl GetPredicate for HashMap<PredicateAddress, Arc<Predicate>> {
    fn get_predicate(&self, addr: &PredicateAddress) -> Arc<Predicate> {
        self[addr].clone()
    }
}

impl GetProgram for HashMap<ContentAddress, Arc<Program>> {
    fn get_program(&self, ca: &ContentAddress) -> Arc<Program> {
        self[ca].clone()
    }
}

impl<T: GetPredicate> GetPredicate for Arc<T> {
    fn get_predicate(&self, addr: &PredicateAddress) -> Arc<Predicate> {
        (**self).get_predicate(addr)
    }
}

impl<T: GetProgram> GetProgram for Arc<T> {
    fn get_program(&self, ca: &ContentAddress) -> Arc<Program> {
        (**self).get_program(ca)
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
/// For each of the solution's `data` elements, we load the associated predicate and
/// its programs and execute each asynchronously in topological order. The leaf nodes
/// are treated as constraints and if any constraint returns `false`, the solution is
/// considered to be invalid.
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
///
/// Returns the total gas spent.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub async fn check_predicates<SA, SB>(
    pre_state: &SA,
    post_state: &SB,
    solution: Arc<Solution>,
    get_predicate: impl GetPredicate,
    get_program: impl 'static + Clone + GetProgram + Send + Sync,
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
        let predicate = get_predicate.get_predicate(&data.predicate_to_solve);
        let solution = solution.clone();
        let pre_state: SA = pre_state.clone();
        let post_state: SB = post_state.clone();
        let config = config.clone();
        let get_program = get_program.clone();

        let future = async move {
            let pre_state = pre_state;
            let post_state = post_state;
            let res = check_predicate(
                &pre_state,
                &post_state,
                solution,
                predicate,
                &get_program,
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
/// Spawns a task for each of the predicate's nodes to execute asynchronously.
/// Oneshot channels are used to provide the execution results from parent to child.
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
    get_program: &impl GetProgram,
    solution_data_index: SolutionDataIndex,
    config: &CheckPredicateConfig,
) -> Result<Gas, PredicateError<SA::Error>>
where
    SA: Clone + StateRead + Send + Sync + 'static,
    SB: Clone + StateRead<Error = SA::Error> + Send + Sync + 'static,
    SA::Future: Send,
    SB::Future: Send,
    SA::Error: Send,
{
    type NodeIx = usize;
    type ParentResultRxs = Vec<oneshot::Receiver<Arc<(Stack, Memory)>>>;

    // A map for providing result channels from parents to their children.
    let mut parent_results: HashMap<NodeIx, ParentResultRxs> = HashMap::new();

    // Prepare the program futures, or return early with an error
    // if the predicate's graph is invalid.
    let program_futures = predicate
        .nodes
        .iter()
        .enumerate()
        .map(|(node_ix, node)| {
            let edges = predicate
                .node_edges(node_ix)
                .ok_or_else(|| PredicateError::InvalidNodeEdges(node_ix))?;

            // Take the channels for our parent results.
            let parents: ParentResultRxs = parent_results.remove(&node_ix).unwrap_or_default();

            // Create the one shot channels for making this node's program results
            // available to its children.
            let mut txs = vec![];
            for &e in edges {
                let (tx, rx) = oneshot::channel();
                txs.push(tx);
                let child = usize::from(e);
                parent_results.entry(child).or_default().push(rx);
            }

            // Map and evaluate the program asynchronously.
            let program_fut = run_program(
                pre_state.clone(),
                post_state.clone(),
                solution.clone(),
                solution_data_index,
                get_program.get_program(&node.program_address),
                ProgramCtx {
                    parents,
                    children: txs,
                    reads: node.reads,
                },
            );

            Ok((node_ix, program_fut))
        })
        .collect::<Result<Vec<(NodeIx, _)>, PredicateError<SA::Error>>>()?;

    // Spawn a task for each program future with a tokio `JoinSet`.
    let mut program_tasks: JoinSet<(NodeIx, Result<_, _>)> = program_futures
        .into_iter()
        .map(|(node_ix, program_fut)| async move { (node_ix, program_fut.await) })
        .collect();

    // Prepare to collect failed programs and unsatisfied constraints.
    let mut failed = Vec::new();
    let mut unsatisfied = Vec::new();

    // Await the successful completion of our programs.
    let mut total_gas: Gas = 0;
    while let Some(join_res) = program_tasks.join_next().await {
        let (node_ix, prog_res) = join_res?;
        match prog_res {
            Ok((satisfied, gas)) => {
                // Check for unsatisfied constraints.
                if let Some(false) = satisfied {
                    unsatisfied.push(node_ix);
                }
                total_gas = total_gas.saturating_add(gas);
            }
            Err(err) => {
                failed.push((node_ix, err));
                if !config.collect_all_failures {
                    break;
                }
            }
        }
    }

    // If there are any failed constraints, return an error.
    if !failed.is_empty() {
        return Err(ProgramErrors(failed).into());
    }

    // If there are any unsatisfied constraints, return an error.
    if !unsatisfied.is_empty() {
        return Err(ConstraintsUnsatisfied(unsatisfied).into());
    }

    Ok(total_gas)
}

/// Map the given program's bytecode and evaluate it.
///
/// If the program is a constraint, returns `Some(bool)` indicating whether or not the constraint
/// was satisfied, otherwise returns `None`.
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(
        fields(CA = %format!("{}:{:?}", &format!("{}", content_addr(&*program))[0..8], ctx.reads)),
        skip_all,
    ),
)]
async fn run_program<SA, SB>(
    pre_state: SA,
    post_state: SB,
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
    program: Arc<Program>,
    ctx: ProgramCtx,
) -> Result<(Option<bool>, Gas), ProgramError<SA::Error>>
where
    SA: StateRead,
    SB: StateRead<Error = SA::Error>,
{
    let program_mapped = BytecodeMapped::try_from(&program.0[..])?;

    // Create a new VM.
    let mut vm = vm::Vm::default();

    #[cfg(feature = "tracing")]
    tracing::trace!(
        "Program {} [{} {}, {} {}]",
        content_addr(&*program),
        ctx.parents.len(),
        if ctx.parents.len() == 1 {
            "parent"
        } else {
            "parents"
        },
        ctx.children.len(),
        if ctx.children.len() == 1 {
            "child"
        } else {
            "children"
        },
    );

    // Use the results of the parent execution to initialise our stack and memory.
    for parent_rx in ctx.parents {
        let parent_result: Arc<_> = parent_rx.await?;
        let (parent_stack, parent_memory) = Arc::unwrap_or_clone(parent_result);
        // Extend the stack.
        let mut stack: Vec<Word> = std::mem::take(&mut vm.stack).into();
        stack.append(&mut parent_stack.into());
        vm.stack = stack.try_into()?;

        // Extend the memory.
        let mut memory: Vec<Word> = std::mem::take(&mut vm.memory).into();
        memory.append(&mut parent_memory.into());
        vm.memory = memory.try_into()?;
    }

    #[cfg(feature = "tracing")]
    tracing::trace!(
        "VM initialised with: \n  ├── {:?}\n  └── {:?}",
        &vm.stack,
        &vm.memory
    );

    // Setup solution data access for execution.
    let mut_keys = vm::mut_keys_set(&solution, solution_data_index);
    let access = Access::new(&solution, solution_data_index, &mut_keys);

    // FIXME: Provide these from Config.
    let gas_cost = |_: &asm::Op| 1;
    let gas_limit = GasLimit::UNLIMITED;

    // Read the state into the VM's memory.
    let gas_spent = match ctx.reads {
        Reads::Pre => {
            vm.exec_bytecode(&program_mapped, access, &pre_state, &gas_cost, gas_limit)
                .await?
        }
        Reads::Post => {
            vm.exec_bytecode(&program_mapped, access, &post_state, &gas_cost, gas_limit)
                .await?
        }
    };

    // If this node is a constraint (has no children), check the stack result.
    let opt_satisfied = if ctx.children.is_empty() {
        Some(vm.stack[..] == [1])
    } else {
        let output = Arc::new((vm.stack, vm.memory));
        for tx in ctx.children {
            let _ = tx.send(output.clone());
        }
        None
    };

    Ok((opt_satisfied, gas_spent))
}
