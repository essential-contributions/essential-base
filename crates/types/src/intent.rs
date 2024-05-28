//! # Intents
//! Types needed to represent an intent.

use crate::{serde::bytecode, ConstraintBytecode, Signature, StateReadBytecode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

/// A set of intents whose content address has been signed.
///
/// For a shorthand constructor, see the downstream
/// `essential_sign::intent_set::sign` function.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SignedSet {
    /// The set of intents whose content address has been signed.
    pub set: Vec<Intent>,
    /// A signature over the intent set's content address.
    ///
    /// This signature must be produced by signing the intent set's
    /// [`ContentAddress`][crate::ContentAddress]. The intent set's
    /// content address can be produced using one of the downstream
    /// `essential_hash::intent_set_addr` functions.
    pub signature: Signature,
}

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
