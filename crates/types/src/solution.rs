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
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// The data the solver is required to provide to solve an intent.
pub struct SolutionData {
    /// Which intent this input data is for.
    pub intent_to_solve: IntentAddress,
    /// The decision variables for the intent.
    pub decision_variables: Vec<Word>,
    /// The transient data being proposed.
    pub transient_data: Vec<Mutation>,
    /// The state mutations being proposed.
    pub state_mutations: Vec<Mutation>,
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

impl Solution {
    /// Get the length of all the state mutations in the solution.
    pub fn state_mutations_len(&self) -> usize {
        self.data.iter().map(|d| d.state_mutations.len()).sum()
    }

    /// Get the length of all the transient data in the solution.
    pub fn transient_data_len(&self) -> usize {
        self.data.iter().map(|d| d.transient_data.len()).sum()
    }
}
