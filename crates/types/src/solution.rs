//! # Solutions
//! Data types that are used to create solutions to intents.

use serde::{Deserialize, Serialize};

use crate::{IntentAddress, Key, Word};

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
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// The data the solver is required to provide to solve an intent.
pub struct SolutionData {
    /// Which intent this input data is for.
    pub intent_to_solve: IntentAddress,
    /// The decision variables for the intent.
    pub decision_variables: Vec<Word>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A mutation to a single key in state.
pub struct Mutation {
    /// Key of state.
    pub key: Key,
    /// Value to set the key to.
    /// None means the value is being deleted.
    pub value: Vec<Word>,
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
