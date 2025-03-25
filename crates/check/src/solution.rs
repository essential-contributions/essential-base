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
use essential_types::{predicate::Program, ContentAddress};
use essential_vm::StateReads;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    sync::Arc,
};
use thiserror::Error;

use rayon::prelude::*;

#[cfg(test)]
mod tests;
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

/// Program node with its parents and inputs.
#[derive(Debug)]
struct Node {
    /// Required parents for this node.
    parents: BTreeMap<u16, Option<Arc<(Stack, Memory)>>>,
    /// Program address.
    program: ContentAddress,
}

impl Node {
    /// Node has all its inputs.
    fn has_all_inputs(&self) -> bool {
        self.parents.iter().all(|(_, v)| v.is_some())
    }
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

impl<E: fmt::Display> fmt::Display for PredicateErrors<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("predicate checking failed for one or more solutions:\n")?;
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

/// Check the given solution set against the given predicates and
/// and compute the post state mutations for this set.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub fn check_and_compute_solution_set<S>(
    state: &S,
    solution_set: SolutionSet,
    get_predicate: impl GetPredicate + Sync,
    get_program: impl 'static + Clone + GetProgram + Send + Sync,
    config: Arc<CheckPredicateConfig>,
) -> Result<(Gas, SolutionSet), PredicatesError<S::Error>>
where
    S: Clone + StateReads + Send + Sync + 'static,
    S::Error: Send,
{
    // Check the set and gather any outputs.
    let set = Arc::new(solution_set);
    let outputs = check_set_predicates(state, set.clone(), get_predicate, get_program, config)?;

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
) -> Result<Outputs, PredicatesError<S::Error>>
where
    S: Clone + StateReads + Send + Sync + 'static,
    S::Error: Send,
{
    #[cfg(feature = "tracing")]
    tracing::trace!("{}", essential_hash::content_addr(&*solution_set));

    // Check each solution in parallel.
    let (ok, failed): (Vec<_>, Vec<_>) = solution_set
        .solutions
        .par_iter()
        .enumerate()
        .map(|(solution_index, solution)| {
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
            );

            match res {
                Ok(ok) => Ok((solution_index as u16, ok)),
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
        .map(|(solution_index, (gas, data_outputs))| {
            let output = DataFromSolution {
                solution_index,
                data: data_outputs,
            };
            total_gas = total_gas.saturating_add(gas);
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
) -> Result<(Gas, Vec<DataOutput>), PredicateError<S::Error>>
where
    S: Clone + StateReads + Send + Sync + 'static,
    S::Error: Send,
{
    let p = predicate.clone();

    // Run all nodes that have all their inputs in parallel
    let run = |(ix, node): (&u16, &Node)| {
        let program = get_program.get_program(&node.program);
        let ctx = ProgramCtx {
            parents: node.parents.values().cloned().map(Option::unwrap).collect(),
            leaf: predicate
                .node_edges(*ix as usize)
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
        (*ix, res)
    };

    check_predicate_inner(run, p, config)
}

fn check_predicate_inner<F, E>(
    run: F,
    predicate: Arc<Predicate>,
    config: &CheckPredicateConfig,
) -> Result<(Gas, Vec<DataOutput>), PredicateError<E>>
where
    F: Fn((&u16, &Node)) -> (u16, Result<(Output, u64), ProgramError<E>>) + Send + Sync + Copy,
    E: Send,
{
    // Nodes with their parents and inputs.
    let mut nodes = BTreeMap::new();

    // Create new node
    let new_node = |node_ix: usize| Node {
        parents: BTreeMap::new(),
        program: predicate.nodes[node_ix].program_address.clone(),
    };

    // For each node add it their children's parents set
    for node_ix in 0..predicate.nodes.len() {
        // Insert this node incase it's a root
        nodes
            .entry(node_ix as u16)
            .or_insert_with(|| new_node(node_ix));

        // Add any children
        for edge in predicate
            .node_edges(node_ix)
            .ok_or_else(|| PredicateError::InvalidNodeEdges(node_ix))?
        {
            // Insert the child if it's not already there and then add this node as a parent
            nodes
                .entry(*edge)
                .or_insert_with(|| new_node(*edge as usize))
                .parents
                .insert(node_ix as u16, None);
        }
    }

    // The outputs from a run.
    let mut failed: Vec<(_, _)> = vec![];
    let mut total_gas: Gas = 0;
    let mut unsatisfied = Vec::new();
    let mut data_outputs = Vec::new();

    // While there are nodes to run
    while !nodes.is_empty() {
        let outputs: BTreeMap<u16, Result<(Output, Gas), _>> = if nodes.len() == 1 {
            nodes
                .iter()
                .filter(|(_, n)| n.has_all_inputs())
                .map(run)
                .collect()
        } else {
            nodes
                .par_iter()
                .filter(|(_, n)| n.has_all_inputs())
                .map(run)
                .collect()
        };

        // Remove any nodes that have been run
        for ix in outputs.keys() {
            nodes.remove(ix);
        }

        // Go through each output
        for (output_from, res) in outputs {
            match res {
                // Parent output
                Ok((Output::Parent(o), gas)) => {
                    // Find any nodes that need this output and add it
                    for node in nodes
                        .values_mut()
                        .filter(|n| n.parents.contains_key(&output_from))
                    {
                        node.parents.insert(output_from, Some(o.clone()));
                    }

                    // Add to the total gas
                    total_gas = total_gas.saturating_add(gas);
                }
                // Leaf output
                Ok((Output::Leaf(o), gas)) => {
                    match o {
                        ProgramOutput::Satisfied(false) => {
                            unsatisfied.push(output_from as usize);
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
                    failed.push((output_from as usize, e));

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
    let mut_keys = vm::mut_keys_set(&solution_set, solution_index);
    let access = Access::new(&solution_set, solution_index, &mut_keys);

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
