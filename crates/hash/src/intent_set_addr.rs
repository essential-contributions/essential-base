//! A small collection of helper functions to assist in the calculation of an
//! intent set's content address.
//!
//! See the [`IntentAddress`][essential_types::IntentAddress] documentation for
//! more information on the requirements behind the intent set content address.

use essential_types::{intent::Intent, ContentAddress};

/// Shorthand for the common case of producing an intent set address from an
/// iterator yielding references to [`Intent`]s.
///
/// If you have already calculated the content address for each intent consider
/// using [`from_intent_addrs`] or [`from_intent_addrs_slice`].
pub fn from_intents<'a>(intents: impl IntoIterator<Item = &'a Intent>) -> ContentAddress {
    let intent_addrs = intents.into_iter().map(crate::content_addr);
    from_intent_addrs(intent_addrs)
}

/// Given the intent content address for each intent in the set, produce the
/// intent set's content address.
///
/// This collects all yielded intent content addresses into a `Vec`, sorts them and then
/// hashes the result to produce the intent set address.
///
/// If you have already collected the content address for each intent into a
/// slice, consider [`from_intent_addrs_slice`].
pub fn from_intent_addrs(intent_addrs: impl IntoIterator<Item = ContentAddress>) -> ContentAddress {
    let mut intent_addrs: Vec<_> = intent_addrs.into_iter().collect();
    from_intent_addrs_slice(&mut intent_addrs)
}

/// Given the intent content address for each intent in the set, produce the
/// intent set's content address.
///
/// This first sorts `intent_addrs` before producing the content address of the
/// slice, ensuring that the address maintains "set" semantics (i.e. the order
/// of the content addresses does not matter).
pub fn from_intent_addrs_slice(intent_addrs: &mut [ContentAddress]) -> ContentAddress {
    intent_addrs.sort();
    crate::content_addr(&intent_addrs)
}
