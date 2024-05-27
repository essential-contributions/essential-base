//! Signing, recovery and verification for intent sets.
//!
//! Throughout the `essential` crates, intent sets are represented as
//! `Vec<Intent>`. If we were to sign the `Vec<Intent>` directly, the order of
//! the intents would need to be known in order to recover the signature. To
//! avoid this unnecessary requirement we instead sign over [the content address
//! of the intent set][essential_hash::intent_set_addr].
//!
//! A [`sign_intent_set`] shorthand function is provided to account for this
//! special case.

use essential_types::intent::{self, Intent};
use secp256k1::{PublicKey, SecretKey};

/// Sign over an intent set.
///
/// This first determines the content address of the intent set and signs the
/// content address to produce the signature.
///
/// If the content address of the set is already known, consider signing
/// the content address directly with [`sign_hash`] and then constructing the
/// [`intent::SignedSet`] from its fields.
pub fn sign(set: Vec<Intent>, sk: &SecretKey) -> intent::SignedSet {
    let ca = essential_hash::intent_set_addr::from_intents(&set);
    let signature = crate::sign_hash(ca.0, sk);
    intent::SignedSet { set, signature }
}

/// Verifies the signature against the content address of the intent set.
pub fn verify(signed: &intent::SignedSet) -> Result<(), secp256k1::Error> {
    let ca = essential_hash::intent_set_addr::from_intents(&signed.set);
    crate::verify_hash(ca.0, &signed.signature)
}

/// Recovers the public key with which the given intent set was signed.
///
/// This first determines the content address of the intent set and recovers
/// over the content address.
///
/// If the content address of the set is already known, consider recovering the
/// content address directly.
pub fn recover(signed: &intent::SignedSet) -> Result<PublicKey, secp256k1::Error> {
    let ca = essential_hash::intent_set_addr::from_intents(&signed.set);
    crate::recover_hash(ca.0, &signed.signature)
}
