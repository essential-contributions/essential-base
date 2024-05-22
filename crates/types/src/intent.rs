//! # Intents
//! Types needed to represent an intent.

use serde::{Deserialize, Serialize};

use crate::{serde::bytecode, ConstraintBytecode, StateReadBytecode};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// An individual intent to be solved.
pub struct Intent {
    /// The programs that read state.
    #[serde(
        serialize_with = "bytecode::serialize_vec",
        deserialize_with = "bytecode::deserialize_vec"
    )]
    pub state_read: Vec<StateReadBytecode>,
    /// The programs that check constraints.
    #[serde(
        serialize_with = "bytecode::serialize_vec",
        deserialize_with = "bytecode::deserialize_vec"
    )]
    pub constraints: Vec<ConstraintBytecode>,
    /// The directive for the intent.
    pub directive: Directive,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// The directive for the intent.
pub enum Directive {
    /// All constraints must be satisfied.
    Satisfy,
    /// Maximize the objective value.
    Maximize(ConstraintBytecode),
    /// Minimize the objective value.
    Minimize(ConstraintBytecode),
}
