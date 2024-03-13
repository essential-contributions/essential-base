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

/// Hash encoded as a 32 byte array.
pub type Hash = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Content address of an intent or set of intents.
pub struct IntentAddress(pub Hash);

/// Single unit of data.
pub type Word = i64;

/// Key for state data.
pub type Key = [Word; 4];

/// Range of keys for state data.
pub type KeyRange = Range<Key>;

/// Externally owned account.
/// Note this type will likely change in the future.
pub type Eoa = [Word; 4];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Address of a persistent intent.
pub struct PersistentAddress {
    /// Content address of the set of intents that this intent is deployed with.
    pub set: IntentAddress,
    /// Content address of the intent.
    pub intent: IntentAddress,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// The address of an intent.
pub enum SourceAddress {
    /// Transient intent content address.
    Transient(IntentAddress),
    /// Persistent intent content address.
    Persistent(PersistentAddress),
}

impl SourceAddress {
    /// Construct a persistent source address.
    pub fn persistent(set: IntentAddress, intent: IntentAddress) -> Self {
        SourceAddress::Persistent(PersistentAddress { set, intent })
    }

    /// Construct a transient source address.
    pub fn transient(intent: IntentAddress) -> Self {
        SourceAddress::Transient(intent)
    }

    /// Get the content address of the set of intents that this intent is deployed with.
    /// For transient intents, this is the same as the intent address.
    pub fn set_address(&self) -> &IntentAddress {
        match self {
            SourceAddress::Transient(intent) => intent,
            SourceAddress::Persistent(persistent) => &persistent.set,
        }
    }

    /// Get the content address of the actual intent.
    pub fn intent_address(&self) -> &IntentAddress {
        match self {
            SourceAddress::Transient(intent) => intent,
            SourceAddress::Persistent(persistent) => &persistent.intent,
        }
    }
}
