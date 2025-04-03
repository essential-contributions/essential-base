//! Items related to validating `Solution`s and `SolutionSet`s.

use crate::{
    types::{
        predicate::Predicate,
        solution::{Solution, SolutionIndex, SolutionSet},
        Key, PredicateAddress, Word,
    },
    vm::{
        self,
        asm::{self, FromBytesError},
        Access, Gas, GasLimit, Memory, Stack,
    },
};
#[cfg(feature = "tracing")]
use essential_hash::content_addr;
use essential_types::{predicate::Program, ContentAddress, Value};
use essential_vm::{StateRead, StateReads};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    sync::Arc,
};
use thiserror::Error;

use rayon::prelude::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_graph_ops;

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

/// Required impl for retrieving access to any [`Solution`]'s [`Predicate`]s during check.
pub trait GetPredicate {
    /// Provides immediate access to the predicate with the given content address.
    ///
    /// This is called by [`check_set_predicates`] for each predicate in each solution being checked.
    ///
    /// All necessary programs are assumed to have been read from storage and
    /// validated ahead of time.
    fn get_predicate(&self, addr: &PredicateAddress) -> Arc<Predicate>;
}

/// Required impl for retrieving access to any [`Predicate`]'s [`Program`]s during check.
pub trait GetProgram {
    /// Provides immediate access to the program with the given content address.
    ///
    /// This is called by [`check_set_predicates`] for each node within each predicate for
    /// each solution being checked.
    ///
    /// All necessary programs are assumed to have been read from storage and
    /// validated ahead of time.
    fn get_program(&self, ca: &ContentAddress) -> Arc<Program>;
}

#[derive(Debug)]
/// Context for checking a predicate
pub struct Ctx<'a> {
    /// The mode the check is running in.
    pub run_mode: RunMode,
    /// The global cache of outputs, indexed by node index.
    pub cache: &'a mut Cache,
}

/// Cache of parent outputs, indexed by node index for a predicate.
pub type Cache = HashMap<u16, Arc<(Stack, Memory)>>;

/// The node context in which a `Program` is evaluated (see [`run_program`]).
struct ProgramCtx {
    /// The outputs from the parent nodes.
    parents: Vec<Arc<(Stack, Memory)>>,
    /// If this node is a leaf.
    leaf: bool,
}

/// The outputs of checking a solution set.
#[derive(Debug, PartialEq)]
pub struct Outputs {
    /// The total gas spent.
    pub gas: Gas,
    /// The data outputs from solving each predicate.
    pub data: Vec<DataFromSolution>,
}

/// The data outputs from solving a particular predicate.
#[derive(Debug, PartialEq)]
pub struct DataFromSolution {
    /// The index of the solution that produced this data.
    pub solution_index: SolutionIndex,
    /// The data output from the solution.
    pub data: Vec<DataOutput>,
}

/// The output of a program execution.
#[derive(Debug, PartialEq)]
enum ProgramOutput {
    /// The program output is a boolean value
    /// indicating whether the constraint was satisfied.
    Satisfied(bool),
    /// The program output is data.
    DataOutput(DataOutput),
}

/// Types of data output from a program.
#[derive(Debug, PartialEq)]
pub enum DataOutput {
    /// The program output is the memory.
    Memory(Memory),
}

/// The output of a program depends on
/// whether it is a leaf or a parent.
enum Output {
    /// Leaf nodes output bools or data.
    Leaf(ProgramOutput),
    /// Parent nodes output a stack and memory.
    Parent(Arc<(Stack, Memory)>),
}

/// The mode the check is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum RunMode {
    /// Generating outputs
    #[default]
    Outputs,
    /// Checking outputs
    Checks,
}

/// [`check_set`] error.
#[derive(Debug, Error)]
pub enum InvalidSolutionSet {
    /// Invalid solution.
    #[error("invalid solution: {0}")]
    Solution(#[from] InvalidSolution),
    /// State mutations validation failed.
    #[error("state mutations validation failed: {0}")]
    StateMutations(#[from] InvalidSetStateMutations),
}

/// [`check_solutions`] error.
#[derive(Debug, Error)]
pub enum InvalidSolution {
    /// There must be at least one solution.
    #[error("must be at least one solution")]
    Empty,
    /// The number of solutions exceeds the limit.
    #[error("the number of solutions ({0}) exceeds the limit ({MAX_SOLUTIONS})")]
    TooMany(usize),
    /// A solution's predicate data length exceeds the limit.
    #[error("solution {0}'s predicate data length exceeded {1} (limit: {MAX_PREDICATE_DATA})")]
    PredicateDataLenExceeded(usize, usize),
    /// Invalid state mutation entry.
    #[error("Invalid state mutation entry: {0}")]
    StateMutationEntry(KvError),
    /// Predicate data value too large.
    #[error("Predicate data value len {0} exceeds limit {MAX_VALUE_SIZE}")]
    PredDataValueTooLarge(usize),
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

/// [`check_set_state_mutations`] error.
#[derive(Debug, Error)]
pub enum InvalidSetStateMutations {
    /// The number of state mutations exceeds the limit.
    #[error("the number of state mutations ({0}) exceeds the limit ({MAX_STATE_MUTATIONS})")]
    TooMany(usize),
    /// Discovered multiple mutations to the same slot.
    #[error("attempt to apply multiple mutations to the same slot: {0:?} {1:?}")]
    MultipleMutationsForSlot(PredicateAddress, Key),
}

/// [`check_set_predicates`] error.
#[derive(Debug, Error)]
pub enum PredicatesError<E> {
    /// One or more solution failed their associated predicate checks.
    #[error("{0}")]
    Failed(#[from] PredicateErrors<E>),
    /// Summing solution gas resulted in overflow.
    #[error("summing solution gas overflowed")]
    GasOverflowed,
    /// Tried to compute mutations on solution set with existing mutations.
    #[error("tried to compute mutations on solution set with existing mutations")]
    ExistingMutations,
}

/// Predicate checking failed for the solution at the given indices.
#[derive(Debug, Error)]
pub struct PredicateErrors<E>(pub Vec<(SolutionIndex, PredicateError<E>)>);

/// [`check_predicate`] error.
#[derive(Debug, Error)]
pub enum PredicateError<E> {
    /// Failed to retrieve edges for a node, indicating that the predicate's graph is invalid.
    #[error("failed to retrieve edges for node {0} indicating an invalid graph")]
    InvalidNodeEdges(usize),
    /// The execution of one or more programs failed.
    #[error("one or more program execution errors occurred: {0}")]
    ProgramErrors(#[from] ProgramErrors<E>),
    /// One or more of the constraints were unsatisfied.
    #[error("one or more constraints unsatisfied: {0}")]
    ConstraintsUnsatisfied(#[from] ConstraintsUnsatisfied),
    /// One or more of the mutations were invalid.
    #[error(transparent)]
    Mutations(#[from] MutationsError),
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
    /// Concatenating the parent program [`Stack`]s caused an overflow.
    #[error("concatenating parent program `Stack`s caused an overflow: {0}")]
    ParentStackConcatOverflow(#[from] vm::error::StackError),
    /// Concatenating the parent program [`Memory`] slices caused an overflow.
    #[error("concatenating parent program `Memory` slices caused an overflow: {0}")]
    ParentMemoryConcatOverflow(#[from] vm::error::MemoryError),
    /// VM execution resulted in an error.
    #[error("VM execution error: {0}")]
    Vm(#[from] vm::error::ExecError<E>),
}

/// The index of each constraint that was not satisfied.
#[derive(Debug, Error)]
pub struct ConstraintsUnsatisfied(pub Vec<usize>);

/// Error with computing mutations.
#[derive(Debug, Error)]
pub enum MutationsError {
    /// Duplicate mutations for the same key.
    #[error("duplicate mutations for the same key: {0:?}")]
    DuplicateMutations(Key),
    /// Error decoding mutations.
    #[error(transparent)]
    DecodeError(#[from] essential_types::solution::decode::MutationDecodeError),
}

/// Maximum number of predicate data of a solution.
pub const MAX_PREDICATE_DATA: u32 = 100;
/// Maximum number of solutions within a solution set.
pub const MAX_SOLUTIONS: usize = 100;
/// Maximum number of state mutations of a solution.
pub const MAX_STATE_MUTATIONS: usize = 1000;
/// Maximum number of words in a slot value.
pub const MAX_VALUE_SIZE: usize = 10_000;
/// Maximum number of words in a slot key.
pub const MAX_KEY_SIZE: usize = 1000;

impl<E: fmt::Display + fmt::Debug> fmt::Display for PredicateErrors<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("predicate checking failed for one or more solutions:\n")?;
        for (ix, err) in &self.0 {
            f.write_str(&format!("  {ix}: {err}\n"))?;
        }
        Ok(())
    }
}

impl<E: fmt::Display + fmt::Debug> fmt::Display for ProgramErrors<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("the programs at the following node indices failed: \n")?;
        for (node_ix, err) in &self.0 {
            f.write_str(&format!("  {node_ix}: {:#?}\n", err))?;
        }
        Ok(())
    }
}

impl fmt::Display for ConstraintsUnsatisfied {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("the constraints at the following indices returned false: \n")?;
        for ix in &self.0 {
            f.write_str(&format!("  {ix}\n"))?;
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

/// Validate a solution set, to the extent it can be validated without reference to
/// its associated predicates.
///
/// This includes solutions and state mutations.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(solution = %content_addr(set)), err))]
pub fn check_set(set: &SolutionSet) -> Result<(), InvalidSolutionSet> {
    check_solutions(&set.solutions)?;
    check_set_state_mutations(set)?;
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

/// Validate the solution set's slice of [`Solution`]s.
pub fn check_solutions(solutions: &[Solution]) -> Result<(), InvalidSolution> {
    // Validate solution.
    // Ensure that at solution has at least one solution.
    if solutions.is_empty() {
        return Err(InvalidSolution::Empty);
    }
    // Ensure that solution length is below limit length.
    if solutions.len() > MAX_SOLUTIONS {
        return Err(InvalidSolution::TooMany(solutions.len()));
    }

    // Check whether the predicate data length has been exceeded.
    for (solution_ix, solution) in solutions.iter().enumerate() {
        // Ensure the length limit is not exceeded.
        if solution.predicate_data.len() > MAX_PREDICATE_DATA as usize {
            return Err(InvalidSolution::PredicateDataLenExceeded(
                solution_ix,
                solution.predicate_data.len(),
            ));
        }
        for v in &solution.predicate_data {
            check_value_size(v).map_err(|_| InvalidSolution::PredDataValueTooLarge(v.len()))?;
        }
    }
    Ok(())
}

/// Validate the solution set's state mutations.
pub fn check_set_state_mutations(set: &SolutionSet) -> Result<(), InvalidSolutionSet> {
    // Validate state mutations.
    // Ensure that the solution set's state mutations length is below limit length.
    if set.state_mutations_len() > MAX_STATE_MUTATIONS {
        return Err(InvalidSetStateMutations::TooMany(set.state_mutations_len()).into());
    }

    // Ensure that no more than one mutation per slot is proposed.
    for solution in &set.solutions {
        let mut mut_keys = HashSet::new();
        for mutation in &solution.state_mutations {
            if !mut_keys.insert(&mutation.key) {
                return Err(InvalidSetStateMutations::MultipleMutationsForSlot(
                    solution.predicate_to_solve.clone(),
                    mutation.key.clone(),
                )
                .into());
            }
            // Check key length.
            check_key_size(&mutation.key).map_err(InvalidSolution::StateMutationEntry)?;
            // Check value length.
            check_value_size(&mutation.value).map_err(InvalidSolution::StateMutationEntry)?;
        }
    }

    Ok(())
}

fn decode_mutations<E>(
    outputs: Outputs,
    mut set: SolutionSet,
) -> Result<SolutionSet, PredicatesError<E>> {
    // For each output check if there are any state mutations and apply them.
    for output in outputs.data {
        // No two outputs can point to the same solution index.
        // Get the solution that these outputs came from.
        let s = &mut set.solutions[output.solution_index as usize];

        // Set to check for duplicate mutations.
        let mut mut_set = HashSet::new();

        // For each memory output decode the mutations and apply them.
        for data in output.data {
            match data {
                DataOutput::Memory(memory) => {
                    for mutation in essential_types::solution::decode::decode_mutations(&memory)
                        .map_err(|e| {
                            PredicatesError::Failed(PredicateErrors(vec![(
                                output.solution_index,
                                PredicateError::Mutations(MutationsError::DecodeError(e)),
                            )]))
                        })?
                    {
                        // Check for duplicate mutation keys.
                        if !mut_set.insert(mutation.key.clone()) {
                            return Err(PredicatesError::Failed(PredicateErrors(vec![(
                                output.solution_index,
                                PredicateError::Mutations(MutationsError::DuplicateMutations(
                                    mutation.key.clone(),
                                )),
                            )])));
                        }

                        // Apply the mutation.
                        s.state_mutations.push(mutation);
                    }
                }
            }
        }
    }
    Ok(set)
}

/// Internal post state used for mutations.
#[derive(Debug, Default)]
struct PostState {
    /// Contract => Key => Value
    state: HashMap<ContentAddress, HashMap<Key, Value>>,
}

/// Arc wrapper for [`PostState`] to allow for cloning.
/// Must take the same error type as the pre state.
#[derive(Debug, Default)]
struct PostStateArc<E>(Arc<PostState>, std::marker::PhantomData<E>);

impl<E> Clone for PostStateArc<E> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), Default::default())
    }
}

impl<E> StateRead for PostStateArc<E>
where
    E: std::fmt::Display + std::fmt::Debug + Sync + Send,
{
    type Error = E;

    fn key_range(
        &self,
        contract_addr: ContentAddress,
        mut key: Key,
        num_values: usize,
    ) -> Result<Vec<Vec<essential_types::Word>>, Self::Error> {
        let out = self
            .0
            .state
            .get(&contract_addr)
            .map(|state| {
                let mut values = Vec::with_capacity(num_values);
                for _ in 0..num_values {
                    let Some(value) = state.get(&key) else {
                        return values;
                    };
                    values.push(value.clone());
                    let Some(k) = next_key(key) else {
                        return values;
                    };
                    key = k;
                }
                values
            })
            .unwrap_or_default();
        Ok(out)
    }
}

/// Get the next key in the range of keys.
fn next_key(mut key: Key) -> Option<Key> {
    for w in key.iter_mut().rev() {
        match *w {
            Word::MAX => *w = Word::MIN,
            _ => {
                *w += 1;
                return Some(key);
            }
        }
    }
    None
}

/// Check the given solution set against the given predicates and
/// and compute the post state mutations for this set.
///
/// This is a two-pass check. The first pass generates the outputs
/// and does not run any post state reads.
/// The second pass checks the outputs and runs the post state reads.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub fn check_and_compute_solution_set_two_pass<S>(
    state: &S,
    solution_set: SolutionSet,
    get_predicate: impl GetPredicate + Sync + Clone,
    get_program: impl 'static + Clone + GetProgram + Send + Sync,
    config: Arc<CheckPredicateConfig>,
) -> Result<(Gas, SolutionSet), PredicatesError<S::Error>>
where
    S: Clone + StateRead + Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    // Create an empty post state,
    let post_state = PostStateArc::<S::Error>(Arc::new(PostState::default()), Default::default());

    // Create an empty cache.
    let mut cache = HashMap::new();

    // Generate the outputs
    let (mut gas, solution_set) = check_and_compute_solution_set(
        &(state.clone(), post_state.clone()),
        solution_set,
        get_predicate.clone(),
        get_program.clone(),
        config.clone(),
        RunMode::Outputs,
        &mut cache,
    )?;

    // Get the post state back.
    let mut post_state =
        Arc::try_unwrap(post_state.0).expect("post state should have one reference");

    // Apply the state mutations to the post state.
    for solution in &solution_set.solutions {
        for mutation in &solution.state_mutations {
            post_state
                .state
                .entry(solution.predicate_to_solve.contract.clone())
                .or_default()
                .insert(mutation.key.clone(), mutation.value.clone());
        }
    }

    // Put the post state back into an arc.
    let post_state = PostStateArc(Arc::new(post_state), Default::default());

    // Check the outputs
    let (g, solution_set) = check_and_compute_solution_set(
        &(state.clone(), post_state.clone()),
        solution_set,
        get_predicate,
        get_program,
        config,
        RunMode::Checks,
        &mut cache,
    )?;

    // Add the total gas
    gas = gas.saturating_add(g);

    // Return solutions set
    Ok((gas, solution_set))
}

/// Check the given solution set against the given predicates and
/// and compute the post state mutations for this set.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub fn check_and_compute_solution_set<S>(
    state: &S,
    solution_set: SolutionSet,
    get_predicate: impl GetPredicate + Sync,
    get_program: impl 'static + Clone + GetProgram + Send + Sync,
    config: Arc<CheckPredicateConfig>,
    run_mode: RunMode,
    cache: &mut HashMap<SolutionIndex, Cache>,
) -> Result<(Gas, SolutionSet), PredicatesError<S::Error>>
where
    S: Clone + StateReads + Send + Sync + 'static,
    S::Error: Send,
{
    // Check the set and gather any outputs.
    let set = Arc::new(solution_set);
    let outputs = check_set_predicates(
        state,
        set.clone(),
        get_predicate,
        get_program,
        config,
        run_mode,
        cache,
    )?;

    // Safe to unwrap the arc here as we have no other references.
    let set = Arc::try_unwrap(set).expect("set should have one reference");

    // Get the gas
    let gas = outputs.gas;

    let set = decode_mutations(outputs, set)?;

    Ok((gas, set))
}

/// Checks all of a [`SolutionSet`]'s [`Solution`]s against their associated [`Predicate`]s.
///
/// For each solution, we load the associated predicate and its programs and execute each
/// in parallel and in topological order. The leaf nodes are treated as constraints or data outputs and if
/// any constraint returns `false`, the whole solution set is considered to be invalid.
///
/// **NOTE:** This assumes that the given `SolutionSet` and all `Predicate`s have already
/// been independently validated using [`solution::check_set`][check_set] and
/// [`predicate::check`][crate::predicate::check] respectively.
///
/// ## Arguments
///
/// - `pre_state` must provide access to state *prior to* mutations being applied.
/// - `post_state` must provide access to state *post* mutations being applied.
///
/// Returns the total gas spent.
pub fn check_set_predicates<S>(
    state: &S,
    solution_set: Arc<SolutionSet>,
    get_predicate: impl GetPredicate + Sync,
    get_program: impl 'static + Clone + GetProgram + Send + Sync,
    config: Arc<CheckPredicateConfig>,
    run_mode: RunMode,
    cache: &mut HashMap<SolutionIndex, Cache>,
) -> Result<Outputs, PredicatesError<S::Error>>
where
    S: Clone + StateReads + Send + Sync + 'static,
    S::Error: Send,
{
    #[cfg(feature = "tracing")]
    tracing::trace!("{}", essential_hash::content_addr(&*solution_set));

    let caches: Vec<_> = (0..solution_set.solutions.len())
        .map(|i| {
            let cache = cache.entry(i as u16).or_default();
            core::mem::take(cache)
        })
        .collect();
    // Check each solution in parallel.
    let (ok, failed): (Vec<_>, Vec<_>) = solution_set
        .solutions
        .par_iter()
        .zip(caches)
        .enumerate()
        .map(|(solution_index, (solution, mut cache))| {
            let predicate = get_predicate.get_predicate(&solution.predicate_to_solve);
            let solution_set = solution_set.clone();
            let state = state.clone();
            let config = config.clone();
            let get_program = get_program.clone();

            let res = check_predicate(
                &state,
                solution_set,
                predicate,
                get_program,
                solution_index
                    .try_into()
                    .expect("solution index already validated"),
                &config,
                Ctx {
                    run_mode,
                    cache: &mut cache,
                },
            );

            match res {
                Ok(ok) => Ok((solution_index as u16, ok, cache)),
                Err(e) => Err((solution_index as u16, e)),
            }
        })
        .partition(Result::is_ok);

    // If any predicates failed, return an error.
    if !failed.is_empty() {
        return Err(PredicateErrors(failed.into_iter().map(Result::unwrap_err).collect()).into());
    }

    // Calculate gas used.
    let mut total_gas: u64 = 0;
    let outputs = ok
        .into_iter()
        .map(Result::unwrap)
        .map(|(solution_index, (gas, data_outputs), c)| {
            let output = DataFromSolution {
                solution_index,
                data: data_outputs,
            };
            total_gas = total_gas.saturating_add(gas);
            *cache.get_mut(&solution_index).expect("cache should exist") = c;
            output
        })
        .collect();

    Ok(Outputs {
        gas: total_gas,
        data: outputs,
    })
}

/// Checks the predicate of the solution within the given set at the given `solution_index`.
///
/// Spawns a rayon task for each of the predicate's nodes to execute in parallel
/// once their inputs are ready.
///
/// **NOTE:** This assumes that the given `SolutionSet` and `Predicate` have been
/// independently validated using [`solution::check_set`][check_set]
/// and [`predicate::check`][crate::predicate::check] respectively.
///
/// ## Arguments
///
/// - `pre_state` must provide access to state *prior to* mutations being applied.
/// - `post_state` must provide access to state *post* mutations being applied.
/// - `solution_index` represents the solution within `solution_set.solutions` that
///   claims to solve this predicate.
///
/// Returns the total gas spent.
pub fn check_predicate<S>(
    state: &S,
    solution_set: Arc<SolutionSet>,
    predicate: Arc<Predicate>,
    get_program: impl GetProgram + Send + Sync + 'static,
    solution_index: SolutionIndex,
    config: &CheckPredicateConfig,
    ctx: Ctx,
) -> Result<(Gas, Vec<DataOutput>), PredicateError<S::Error>>
where
    S: Clone + StateReads + Send + Sync + 'static,
    S::Error: Send,
{
    let p = predicate.clone();

    // Run all nodes that have all their inputs in parallel
    let run = |ix: u16, parents: Vec<Arc<(Stack, Memory)>>| {
        let program = get_program.get_program(&predicate.nodes[ix as usize].program_address);
        let ctx = ProgramCtx {
            parents,
            leaf: predicate
                .node_edges(ix as usize)
                .expect("This is already checked")
                .is_empty(),
        };
        let res = run_program(
            state.clone(),
            solution_set.clone(),
            solution_index,
            program,
            ctx,
        );
        (ix, res)
    };

    check_predicate_inner(run, p, config, &get_program, ctx)
}

/// Includes nodes with no parents
fn create_parent_map<E>(
    predicate: &Predicate,
) -> Result<BTreeMap<u16, Vec<u16>>, PredicateError<E>> {
    let mut nodes: BTreeMap<u16, Vec<u16>> = BTreeMap::new();
    // For each node add it their children's parents set
    for node_ix in 0..predicate.nodes.len() {
        // Insert this node incase it's a root
        nodes.entry(node_ix as u16).or_default();

        // Add any children
        for edge in predicate
            .node_edges(node_ix)
            .ok_or_else(|| PredicateError::InvalidNodeEdges(node_ix))?
        {
            // Insert the child if it's not already there and then add this node as a parent
            nodes.entry(*edge).or_default().push(node_ix as u16);
        }
    }
    Ok(nodes)
}

fn in_degrees(num_nodes: usize, parent_map: &BTreeMap<u16, Vec<u16>>) -> BTreeMap<u16, usize> {
    let mut in_degrees = BTreeMap::new();
    for node in 0..num_nodes {
        in_degrees.insert(
            node as u16,
            parent_map.get(&(node as u16)).map_or(0, |v| v.len()),
        );
    }

    in_degrees
}

fn reduce_in_degrees(in_degrees: &mut BTreeMap<u16, usize>, children: &[u16]) {
    for child in children {
        if let Some(in_degree) = in_degrees.get_mut(child) {
            *in_degree = in_degree.saturating_sub(1);
        }
    }
}

fn find_nodes_with_no_parents(in_degrees: &BTreeMap<u16, usize>) -> Vec<u16> {
    in_degrees
        .iter()
        .filter_map(
            |(node, in_degree)| {
                if *in_degree == 0 {
                    Some(*node)
                } else {
                    None
                }
            },
        )
        .collect()
}

/// Sorts the nodes in parallel topological order.
///
/// ## Note
/// This is not a perfect ordering as the following:
/// ```text
///   A
///  / \
/// B   C
/// |   |
/// D   E
///  \ /
///   F
/// ```
/// Results in:
/// ```text
/// [[A], [B, C], [D, E], [F]]
/// ```
/// If `B` or `C` finish first then they could start on
/// `D` or `E` respectively but this sort doesn't allow that.
fn parallel_topo_sort<E>(
    predicate: &Predicate,
    parent_map: &BTreeMap<u16, Vec<u16>>,
) -> Result<Vec<Vec<u16>>, PredicateError<E>> {
    let mut in_degrees = in_degrees(predicate.nodes.len(), parent_map);

    let mut out = Vec::new();
    while !in_degrees.is_empty() {
        let current_level = find_nodes_with_no_parents(&in_degrees);
        if current_level.is_empty() {
            // Cycle detected
            // TODO: Change error
            return Err(PredicateError::InvalidNodeEdges(0));
        }

        out.push(current_level.clone());

        for node in current_level {
            let children = predicate
                .node_edges(node as usize)
                .ok_or_else(|| PredicateError::InvalidNodeEdges(node as usize))?;
            reduce_in_degrees(&mut in_degrees, children);
            in_degrees.remove(&node);
        }
    }

    Ok(out)
}

fn find_deferred<F>(predicate: &Predicate, is_deferred: F) -> HashSet<u16>
where
    F: Fn(&essential_types::predicate::Node) -> bool,
{
    let mut deferred = HashSet::new();
    for (ix, node) in predicate.nodes.iter().enumerate() {
        if is_deferred(node) {
            deferred.insert(ix as u16);
        }
        if deferred.contains(&(ix as u16)) {
            for child in predicate.node_edges(ix).expect("Already checked") {
                deferred.insert(*child);
            }
        }
    }
    deferred
}

fn should_cache(node: u16, predicate: &Predicate, deferred: &HashSet<u16>) -> bool {
    !deferred.contains(&node)
        && predicate
            .node_edges(node as usize)
            .expect("Already checked")
            .iter()
            .any(|child| deferred.contains(child))
}

fn remove_deferred(nodes: Vec<Vec<u16>>, deferred: &HashSet<u16>) -> Vec<Vec<u16>> {
    nodes
        .into_iter()
        .map(|level| {
            level
                .into_iter()
                .filter(|node| !deferred.contains(node))
                .collect::<Vec<_>>()
        })
        .filter(|level| !level.is_empty())
        .collect()
}

fn remove_not_deferred(nodes: Vec<Vec<u16>>, deferred: &HashSet<u16>) -> Vec<Vec<u16>> {
    nodes
        .into_iter()
        .map(|level| {
            level
                .into_iter()
                .filter(|node| deferred.contains(node))
                .collect::<Vec<_>>()
        })
        .filter(|level| !level.is_empty())
        .collect()
}

/// Handles the checking of a predicate.
/// - Sorts the nodes into parallel topological order.
/// - Sets up for the run type.
/// - Runs the programs in parallel where appropriate.
/// - Collects the outputs and gas.
fn check_predicate_inner<F, E>(
    run: F,
    predicate: Arc<Predicate>,
    config: &CheckPredicateConfig,
    get_program: &(impl GetProgram + Send + Sync + 'static),
    ctx: Ctx<'_>,
) -> Result<(Gas, Vec<DataOutput>), PredicateError<E>>
where
    F: Fn(u16, Vec<Arc<(Stack, Memory)>>) -> (u16, Result<(Output, u64), ProgramError<E>>)
        + Send
        + Sync
        + Copy,
    E: Send + std::fmt::Display,
{
    // Get the mode we are running and the global cache.
    let Ctx { run_mode, cache } = ctx;

    // Create the parent map
    let parent_map = create_parent_map(&predicate)?;

    // Create a parallel topological sort of the nodes
    let sorted_nodes = parallel_topo_sort(&predicate, &parent_map)?;

    // Filter for which nodes are deferred. This is nodes with a post state read.
    let deferred_filter = |node: &essential_types::predicate::Node| -> bool {
        asm::effects::bytes_contains_any(
            &get_program.get_program(&node.program_address).0,
            asm::effects::Effects::PostKeyRange | asm::effects::Effects::PostKeyRangeExtern,
        )
    };

    // Get the set of deferred nodes.
    let deferred = find_deferred(&predicate, deferred_filter);

    // Depending on the run mode remove the deferred nodes or other nodes.
    let sorted_nodes = match run_mode {
        RunMode::Outputs => remove_deferred(sorted_nodes, &deferred),
        RunMode::Checks => remove_not_deferred(sorted_nodes, &deferred),
    };

    // Setup a local cache for the outputs.
    let mut local_cache = Cache::new();

    // The outputs from a run.
    let mut failed: Vec<(_, _)> = vec![];
    let mut total_gas: Gas = 0;
    let mut unsatisfied = Vec::new();
    let mut data_outputs = Vec::new();

    // Run each set of parallel nodes.
    for parallel_nodes in sorted_nodes {
        // Run 1 or no length in serial to avoid overhead.
        let outputs: BTreeMap<u16, Result<(Output, Gas), _>> =
            if parallel_nodes.len() == 1 || parallel_nodes.is_empty() {
                parallel_nodes
                    .into_iter()
                    .map(|ix| {
                        // Check global cache then local cache
                        // for parent inputs.
                        let inputs = parent_map[&ix]
                            .iter()
                            .filter_map(|parent_ix| {
                                cache
                                    .get(parent_ix)
                                    .cloned()
                                    .or_else(|| local_cache.get(parent_ix).cloned())
                            })
                            .collect();

                        // Run the program.
                        run(ix, inputs)
                    })
                    .collect()
            } else {
                parallel_nodes
                    .into_par_iter()
                    .map(|ix| {
                        // Check global cache then local cache
                        // for parent inputs.
                        let inputs = parent_map[&ix]
                            .iter()
                            .filter_map(|parent_ix| {
                                cache
                                    .get(parent_ix)
                                    .cloned()
                                    .or_else(|| local_cache.get(parent_ix).cloned())
                            })
                            .collect();

                        // Run the program.
                        run(ix, inputs)
                    })
                    .collect()
            };
        for (node, res) in outputs {
            match res {
                Ok((Output::Parent(o), gas)) => {
                    // Check if we should add this output to the global or local cache.
                    if should_cache(node, &predicate, &deferred) {
                        cache.insert(node, o.clone());
                    } else {
                        local_cache.insert(node, o.clone());
                    }

                    // Add to the total gas
                    total_gas = total_gas.saturating_add(gas);
                }
                Ok((Output::Leaf(o), gas)) => {
                    match o {
                        ProgramOutput::Satisfied(false) => {
                            unsatisfied.push(node as usize);
                        }
                        ProgramOutput::Satisfied(true) => {
                            // Nothing to do here.
                        }
                        ProgramOutput::DataOutput(data_output) => {
                            data_outputs.push(data_output);
                        }
                    }

                    // Add to the total gas
                    total_gas = total_gas.saturating_add(gas);
                }
                Err(e) => {
                    failed.push((node as usize, e));

                    if !config.collect_all_failures {
                        return Err(ProgramErrors(failed).into());
                    }
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

    Ok((total_gas, data_outputs))
}

/// Map the given program's bytecode and evaluate it.
///
/// If the program is a constraint, returns `Some(bool)` indicating whether or not the constraint
/// was satisfied, otherwise returns `None`.
fn run_program<S>(
    state: S,
    solution_set: Arc<SolutionSet>,
    solution_index: SolutionIndex,
    program: Arc<Program>,
    ctx: ProgramCtx,
) -> Result<(Output, Gas), ProgramError<S::Error>>
where
    S: StateReads,
{
    let ProgramCtx { parents, leaf } = ctx;

    // Pull ops into memory.
    let ops = asm::from_bytes(program.0.iter().copied()).collect::<Result<Vec<_>, _>>()?;

    // Create a new VM.
    let mut vm = vm::Vm::default();

    // Use the results of the parent execution to initialise our stack and memory.
    for parent_result in parents {
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

    // Setup solution access for execution.
    let access = Access::new(Arc::new(solution_set.solutions.clone()), solution_index);

    // FIXME: Provide these from Config.
    let gas_cost = |_: &asm::Op| 1;
    let gas_limit = GasLimit::UNLIMITED;

    // Read the state into the VM's memory.
    let gas_spent = vm.exec_ops(&ops, access, &state, &gas_cost, gas_limit)?;

    let out = if leaf {
        match vm.stack[..] {
            [2] => Output::Leaf(ProgramOutput::DataOutput(DataOutput::Memory(vm.memory))),
            [1] => Output::Leaf(ProgramOutput::Satisfied(true)),
            _ => Output::Leaf(ProgramOutput::Satisfied(false)),
        }
    } else {
        let output = Arc::new((vm.stack, vm.memory));
        Output::Parent(output)
    };

    Ok((out, gas_spent))
}
