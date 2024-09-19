//! A small collection of helper functions to assist in the calculation of an
//! contract's content address.
//!
//! See the [`PredicateAddress`][essential_types::PredicateAddress] documentation for
//! more information on the requirements behind the contract content address.

use essential_types::{contract::Contract, ContentAddress, Hash};

/// Shorthand for the common case of producing an contract address from an
/// iterator yielding references to [`Predicate`]s.
///
/// If you have already calculated the content address for each predicate consider
/// using [`from_predicate_addrs`] or [`from_predicate_addrs_slice`].
pub fn from_contract(contract: &Contract) -> ContentAddress {
    let predicate_addrs = contract.predicates.iter().map(crate::content_addr);
    from_predicate_addrs(predicate_addrs, &contract.salt)
}

/// Given the predicate content address for each predicate in the contract, produce the
/// contract's content address.
///
/// This collects all yielded predicate content addresses into a `Vec`, sorts them and then
/// hashes the result to produce the contract address.
///
/// If you have already collected the content address for each predicate into a
/// slice, consider [`from_predicate_addrs_slice`].
pub fn from_predicate_addrs(
    predicate_addrs: impl IntoIterator<Item = ContentAddress>,
    salt: &Hash,
) -> ContentAddress {
    let mut predicate_addrs: Vec<_> = predicate_addrs.into_iter().collect();
    from_predicate_addrs_slice(&mut predicate_addrs, salt)
}

/// Given the predicate content address for each predicate in the contract, produce the
/// contract's content address.
///
/// This first sorts `predicate_addrs` before producing the content address of the
/// slice, ensuring that the address maintains "contract" semantics (i.e. the order
/// of the content addresses does not matter).
pub fn from_predicate_addrs_slice(
    predicate_addrs: &mut [ContentAddress],
    salt: &Hash,
) -> ContentAddress {
    predicate_addrs.sort();
    ContentAddress(crate::hash_bytes_iter(
        predicate_addrs
            .iter()
            .map(|addr| addr.0.as_slice())
            .chain(Some(salt.as_slice())),
    ))
}
