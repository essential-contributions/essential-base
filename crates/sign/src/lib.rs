//! A minimal crate providing Essential's generic signing, verification
//! and public key recovery functions implemented using [`secp256k1`] and the
//! [`essential_hash`] crate.
//!
//! ## Signing Arbitrary Data
//!
//! For signing arbitrary data, the following take care of hashing the data in a
//! consistent manner internally.
//!
//! - [`sign`]
//! - [`verify`]
//! - [`recover`]
//!
//! ## Signing Hashes
//!
//! In cases where the `Hash` (or `ContentAddress`) is already known, the following
//! are more efficient as they avoid hashing a second time:
//!
//! - [`sign_hash`]
//! - [`verify_hash`]
//! - [`recover_hash`]

#![deny(missing_docs)]
#![deny(unsafe_code)]

use essential_hash::hash;
use essential_types::{Hash, Signature, Signed};
pub use secp256k1;
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1, SecretKey,
};
use serde::Serialize;

pub mod intent_set;

/// Sign over data with secret key using secp256k1 curve.
///
/// This first hashes the given data, then produces a signature over the hash using [`sign_hash`].
pub fn sign<T: Serialize>(data: T, sk: &SecretKey) -> Signed<T> {
    let hash = hash(&data);
    let signature = sign_hash(hash, sk);
    Signed { data, signature }
}

/// Sign directly over a sha256 hash (as produced by [`essential_hash::hash`])
/// with the given secret key using `secp256k1`.
///
/// This treats the hash as a digest from which a [`Message`] is produced and then signed.
pub fn sign_hash(hash: Hash, sk: &SecretKey) -> Signature {
    let message = Message::from_digest(hash);
    sign_message(&message, sk)
}

/// Sign directly over the given [`Message`] with secret key using secp256k1 curve.
pub fn sign_message(msg: &Message, sk: &SecretKey) -> Signature {
    let secp = Secp256k1::new();
    let (rec_id, sig) = secp.sign_ecdsa_recoverable(msg, sk).serialize_compact();
    Signature(sig, rec_id.to_i32().try_into().unwrap())
}

/// Verify signature against data.
///
/// This first hashes the `Signed.data` field then calls `verify_hash` with the given signature.
pub fn verify<T: Serialize>(signed: &Signed<T>) -> Result<(), secp256k1::Error> {
    let hash = hash(&signed.data);
    verify_hash(hash, &signed.signature)
}

/// Verify a signature over the given `sha256` hash.
///
/// This treats the given hash as a digest for a [`Message`] that is verified
/// with [`verify_message`].
pub fn verify_hash(hash: Hash, signature: &Signature) -> Result<(), secp256k1::Error> {
    let msg = Message::from_digest(hash);
    verify_message(&msg, signature)
}

/// Verify the given message against the given signature.
pub fn verify_message(msg: &Message, signature: &Signature) -> Result<(), secp256k1::Error> {
    let pk = recover_from_message(msg, signature)?;
    let secp = Secp256k1::new();
    let sig = secp256k1::ecdsa::Signature::from_compact(&signature.0)?;
    secp.verify_ecdsa(msg, &sig, &pk)
}

/// Recover the [`PublicKey`] from the given signed data.
///
/// This first hashes the given `Signed.data`, then calls [`recover_hash`] with
/// the given signature.
pub fn recover<T: Serialize>(signed: Signed<T>) -> Result<PublicKey, secp256k1::Error> {
    let hash = hash(&signed.data);
    recover_hash(hash, &signed.signature)
}

/// Recover the [`PublicKey`] from the signed sha256 hash.
///
/// This treats the given hash as a digest for a [`Message`], then uses [`recover_from_message`].
pub fn recover_hash(hash: Hash, signature: &Signature) -> Result<PublicKey, secp256k1::Error> {
    let msg = Message::from_digest(hash);
    recover_from_message(&msg, signature)
}

/// Recover public key from signed `secp256k1::Message` and `Signature`
pub fn recover_from_message(
    message: &Message,
    signature: &Signature,
) -> Result<PublicKey, secp256k1::Error> {
    let recovery_id = RecoveryId::from_i32(i32::from(signature.1))?;
    let recoverable_signature = RecoverableSignature::from_compact(&signature.0, recovery_id)?;
    let secp = Secp256k1::new();
    let public_key = secp.recover_ecdsa(message, &recoverable_signature)?;
    Ok(public_key)
}
