#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! # Common types for Essential Chain.

use core::time::Duration;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use solution::Solution;

pub mod convert;
pub mod fmt;
pub mod intent;
pub mod signature_ser;
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

/// Hash encoded as a 32 byte array.
pub type Hash = [u8; 32];

/// Externally owned account.
pub type Eoa = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Recoverable ECDSA signature over some data.
pub struct Signature(
    /// Compact signature
    #[serde(with = "signature_ser")]
    pub [u8; 64],
    /// ID used for public key recovery
    pub u8,
);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// Content address of an intent or set of intents.
pub struct ContentAddress(pub Hash);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// Address of a persistent intent.
pub struct IntentAddress {
    /// Content address of the set of intents with which this intent was deployed.
    ///
    /// This is equal to `essential_hash::content_addr(intent_addresses)`,
    /// where `intent_addresses` is a `&[ContentAddress]` sorted by the
    /// `ContentAddress` `Ord` implementation.
    pub set: ContentAddress,
    /// Content address of the intent.
    ///
    /// This is equal to `essential_hash::content_addr(intent)` where `intent`
    /// is a [`&Intent`][intent::Intent].
    pub intent: ContentAddress,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A signed piece of data.
pub struct Signed<T> {
    /// The data that is signed.
    pub data: T,
    /// The signature over the data.
    pub signature: Signature,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A protocol block.
pub struct Block {
    /// The block number.
    pub number: u64,
    /// The timestamp of the block.
    pub timestamp: Duration,
    /// The batch of solutions.
    pub batch: Batch,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A batch of solutions
pub struct Batch {
    /// The solutions in the batch.
    pub solutions: Vec<Signed<Solution>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// The storage layout of a stateful intent.
pub struct StorageLayout;
