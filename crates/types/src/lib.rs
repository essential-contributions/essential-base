#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! # Common types for Essential Chain.

use std::ops::Range;

use serde::{Deserialize, Serialize};

pub mod convert;
pub mod intent;
pub mod slots;
pub mod solution;

/// Constraint code serialized as json.
pub type ConstraintBytecode = Vec<u8>;

/// State read code serialized as json.
pub type StateReadBytecode = Vec<u8>;

/// Single unit of data.
pub type Word = i64;

/// Key for state data.
pub type Key = [Word; 4];

/// Range of keys for state data.
pub type KeyRange = Range<Key>;

/// Hash encoded as a 32 byte array.
pub type Hash = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Content address of an intent or set of intents.
pub struct ContentAddress(pub Hash);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Address of a persistent intent.
pub struct IntentAddress {
    /// Content address of the set of intents that this intent is deployed with.
    pub set: ContentAddress,
    /// Content address of the intent.
    pub intent: ContentAddress,
}
