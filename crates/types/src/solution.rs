//! # Solutions
//! Data types that are used to create solutions to intents.

use serde::{Deserialize, Serialize};

use crate::{ContentAddress, IntentAddress, Key, Signed, Word};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

/// Index into the solution data.
pub type SolutionDataIndex = u16;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A solution to intents.
pub struct Solution {
    /// The input data for each intent.
    pub data: Vec<SolutionData>,
    /// The state mutations being proposed.
    pub state_mutations: Vec<StateMutation>,
    /// Hashes of partial solutions that are required to be a subset of this solution.
    pub partial_solutions: Vec<Signed<ContentAddress>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A partial solution to intents.
pub struct PartialSolution {
    /// The partial input data for each intent.
    pub data: Vec<PartialSolutionData>,
    /// The state mutations being proposed.
    pub state_mutations: Vec<StateMutation>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// The data the solver is required to provide to solve an intent.
pub struct SolutionData {
    /// Which intent this input data is for.
    pub intent_to_solve: IntentAddress,
    /// The decision variables for the intent.
    pub decision_variables: Vec<DecisionVariable>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// The partial data the solver is required to provide to solve an intent.
pub struct PartialSolutionData {
    /// Which intent this input data is for.
    pub intent_to_solve: IntentAddress,
    /// The partial decision variables for the intent.
    pub decision_variables: Vec<Option<DecisionVariable>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A decision variable for a solution.
pub enum DecisionVariable {
    /// An inline decision variable.
    Inline(Word),
    /// A decision variable from another intent in this solution.
    Transient(DecisionVariableIndex),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// Index into the decision variables of a solution data.
pub struct DecisionVariableIndex {
    /// The solution data that this decision variable is from.
    pub solution_data_index: SolutionDataIndex,
    /// The index into the decision variables of the solution data.
    pub variable_index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A mutation to a single key in state.
pub struct Mutation {
    /// Key of state.
    pub key: Key,
    /// Value to set the key to.
    /// None means the value is being deleted.
    pub value: Option<Word>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// The state that is being proposed to be mutated.
/// This state is owned by the persistent intent.
pub struct StateMutation {
    /// A pathway intent to allow a state mutation.
    ///
    /// The intent must be solved to allow the state mutation.
    pub pathway: SolutionDataIndex,
    /// The mutations to the state.
    pub mutations: Vec<Mutation>,
}
