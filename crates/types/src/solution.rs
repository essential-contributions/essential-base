//! # Solutions
//! Data types that are used to create solutions to intents.

use serde::{Deserialize, Serialize};

use crate::{IntentAddress, Key, Value, Word};

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
    /// The transient data being proposed.
    pub transient_data: Vec<Mutations>,
    /// The state mutations being proposed.
    pub state_mutations: Vec<Mutations>,
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
/// A mutation to a single key in state or transient data.
pub struct Mutation {
    /// Key to data.
    pub key: Key,
    /// Value to set the key to.
    /// Empty value means the value is being deleted.
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// The state or transient data that is being proposed to be mutated.
/// This state or transient data is owned by an intent.
pub struct Mutations {
    /// A pathway intent to allow a set of mutations.
    ///
    /// The intent must be solved to allow the mutations.
    pub pathway: SolutionDataIndex,
    /// The mutations to the state or transient data.
    pub mutations: Vec<Mutation>,
}
