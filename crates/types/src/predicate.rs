//! # Predicates
//! Types needed to represent a predicate.

use crate::{serde::bytecode, ConstraintBytecode, StateReadBytecode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// An individual predicate to be solved.
pub struct Predicate {
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
}
