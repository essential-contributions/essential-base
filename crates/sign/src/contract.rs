//! Signing, recovery and verification for contracts.
//!
//! Throughout the `essential` crates, contracts are represented as
//! `Vec<Predicate>`. If we were to sign the `Vec<Predicate>` directly, the order of
//! the predicates would need to be known in order to recover the signature. To
//! avoid this unnecessary requirement we instead sign over [the content address
//! of the contract][essential_hash::contract_addr].
//!
//! A [`contract::sign`][sign] shorthand function is provided to account for this
//! special case.

use essential_types::contract::{self, Contract};
use secp256k1::{PublicKey, SecretKey};

/// Sign over an contract.
///
/// This first determines the content address of the contract and signs the
/// content address to produce the signature.
///
/// If the content address of the contract is already known, consider signing
/// the content address directly with [`sign_hash`][crate::sign_hash] and then
/// constructing the [`predicate::SignedContract`] from its fields.
pub fn sign(contract: Contract, sk: &SecretKey) -> contract::SignedContract {
    let ca = essential_hash::content_addr(&contract);
    let signature = crate::sign_hash(ca.0, sk);
    contract::SignedContract {
        contract,
        signature,
    }
}

/// Verifies the signature against the content address of the contract.
pub fn verify(signed: &contract::SignedContract) -> Result<(), secp256k1::Error> {
    let ca = essential_hash::content_addr(&signed.contract);
    crate::verify_hash(ca.0, &signed.signature)
}

/// Recovers the public key with which the given contract was signed.
///
/// This first determines the content address of the contract and recovers
/// over the content address.
///
/// If the content address of the contract is already known, consider recovering the
/// content address directly.
pub fn recover(signed: &contract::SignedContract) -> Result<PublicKey, secp256k1::Error> {
    let ca = essential_hash::content_addr(&signed.contract);
    crate::recover_hash(ca.0, &signed.signature)
}
