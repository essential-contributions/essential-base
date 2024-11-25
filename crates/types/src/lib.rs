#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! # Common types for Essential Chain.

use ::serde::{Deserialize, Serialize};
#[doc(inline)]
pub use contract::Contract;
use core::time::Duration;
#[doc(inline)]
pub use predicate::{Predicate, Program};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[doc(inline)]
pub use solution::{Solution, SolutionIndex, SolutionSet};

pub mod contract;
pub mod convert;
pub mod fmt;
pub mod predicate;
pub mod serde;
pub mod solution;

/// Essential ASM encoded as bytes.
pub type Bytecode = Vec<u8>;

/// Single unit of data.
pub type Word = i64;

/// Key for data.
pub type Key = Vec<Word>;

/// The data at a key.
pub type Value = Vec<Word>;

/// Hash encoded as a 32 byte array.
pub type Hash = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Recoverable ECDSA signature over some data.
pub struct Signature(
    /// Compact signature
    pub [u8; 64],
    /// ID used for public key recovery
    pub u8,
);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// Content address of a predicate or contract.
pub struct ContentAddress(pub Hash);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// Address of a predicate.
pub struct PredicateAddress {
    /// Content address of the contract with which this predicate was deployed.
    ///
    /// This is equal to `essential_hash::content_addr(predicate_addresses)`,
    /// where `predicate_addresses` is a `&[ContentAddress]` sorted by the
    /// `ContentAddress` `Ord` implementation.
    pub contract: ContentAddress,
    /// Content address of the predicate.
    ///
    /// This is equal to `essential_hash::content_addr(predicate)` where `predicate`
    /// is a [`&Predicate`][predicate::Predicate].
    pub predicate: ContentAddress,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A protocol block.
pub struct Block {
    /// The block number.
    pub number: Word,
    /// The timestamp of the block.
    pub timestamp: Duration,
    /// The list of solution sets that make up a block.
    pub solution_sets: Vec<SolutionSet>,
}
