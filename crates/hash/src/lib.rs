//! A minimal crate containing Essential's [`hash`] function and associated pre-hash
//! generic serialization implementation [`serialize`] based on [`postcard`].

#![deny(missing_docs)]
#![deny(unsafe_code)]

use essential_types::{convert::bytes_from_word, ContentAddress, Hash, Word};
use serde::Serialize;
use sha2::Digest;

mod address_impl;
pub mod block_addr;
pub mod contract_addr;

/// Standardized trait for creating content addresses for
/// types using the correct constructors.
pub trait Address {
    /// Produce the content address for self.
    fn content_address(&self) -> ContentAddress;
}

/// Serialize data for hashing using postcard.
///
/// This serialization format is standardized across essential crates.
/// Attempting to hash data serialized with any other serialization
/// implementation will almost certainly result in a different hash.
pub fn serialize<T: Serialize>(t: &T) -> Vec<u8> {
    postcard::to_allocvec(t).expect("`postcard`'s `Serializer` implementation should never fail")
}

/// Hash data using SHA-256.
///
/// Internally, this first serializes the given type using [`serialize`] then
/// hashes the resulting slice of bytes using the `Sha256` digest.
pub fn hash<T: Serialize>(t: &T) -> Hash {
    let data = serialize(t);
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    hasher.update(&data);
    hasher.finalize().into()
}

/// Shorthand for hashing the given value in order to produce its content address.
///
/// Commonly useful for solutions, predicates and contracts.
pub fn content_addr<T: Address>(t: &T) -> ContentAddress {
    t.content_address()
}

/// Hash words in the same way that `Crypto::Sha256` does.
pub fn hash_words(words: &[Word]) -> Hash {
    let data = words
        .iter()
        .copied()
        .flat_map(bytes_from_word)
        .collect::<Vec<_>>();
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    hasher.update(&data);
    hasher.finalize().into()
}
