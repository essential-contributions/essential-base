#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! # Common types for Essential Chain.

use core::time::Duration;

use serde::{Deserialize, Serialize};
use solution::Solution;

pub mod convert;
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

/// The signature over some data.
pub type Signature = [u8; 64];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Owner of a state key.
pub enum Owner {
    /// An externally owned account.
    Eoa(Eoa),
    /// An intents account.
    Intent(ContentAddress),
}

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// A signed piece of data.
pub struct Signed<T> {
    /// The data that is signed.
    pub data: T,
    /// The signature over the data.
    #[serde(with = "signature_ser")]
    pub signature: Signature,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
/// A batch of solutions
pub struct Batch {
    /// The solutions in the batch.
    pub solutions: Vec<Signed<Solution>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// The storage layout of a stateful intent.
pub struct StorageLayout;
