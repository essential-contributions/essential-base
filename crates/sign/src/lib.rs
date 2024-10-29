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

use essential_types::{Hash, Signature};
pub use secp256k1;
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1, SecretKey,
};

pub mod contract;
pub mod encode;

/// Sign directly over a hash with the given secret key using `secp256k1`.
///
/// This treats the hash as a digest from which a [`Message`] is produced and then signed.
///
/// If you plan to use the resulting `Signature` with [`verify`] or [`recover`]
/// to verify a signature or recover a public key over some arbitrary data, the
/// given `hash` must be produced by [`essential_hash::hash`] (i.e. be a sha256
/// hash).
pub fn sign_hash(hash: Hash, sk: &SecretKey) -> Signature {
    let message = Message::from_digest(hash);
    sign_message(&message, sk)
}

/// Sign directly over the given [`Message`] with secret key using secp256k1 curve.
fn sign_message(msg: &Message, sk: &SecretKey) -> Signature {
    let secp = Secp256k1::new();
    let (rec_id, sig) = secp.sign_ecdsa_recoverable(msg, sk).serialize_compact();
    Signature(sig, i32::from(rec_id).try_into().unwrap())
}

/// Verify a signature over the given hash.
///
/// This treats the given hash as a digest for a [`Message`] that is verified
/// with [`verify_message`].
pub fn verify_hash(hash: Hash, signature: &Signature) -> Result<(), secp256k1::Error> {
    let msg = Message::from_digest(hash);
    verify_message(&msg, signature)
}

/// Verify the given message against the given signature.
fn verify_message(msg: &Message, signature: &Signature) -> Result<(), secp256k1::Error> {
    let pk = recover_from_message(msg, signature)?;
    let secp = Secp256k1::new();
    let sig = secp256k1::ecdsa::Signature::from_compact(&signature.0)?;
    secp.verify_ecdsa(msg, &sig, &pk)
}

/// Recover the [`PublicKey`] from the signed hash.
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
    let recovery_id = RecoveryId::try_from(i32::from(signature.1))?;
    let recoverable_signature = RecoverableSignature::from_compact(&signature.0, recovery_id)?;
    let secp = Secp256k1::new();
    let public_key = secp.recover_ecdsa(message, &recoverable_signature)?;
    Ok(public_key)
}
