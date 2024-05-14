//! A minimal crate providing Essential's generic signing, verification
//! and public key recovery functions implemented using [`secp256k1`] and the
//! [`essential_hash`] crate.
//!
//! Includes [`sign`], [`verify`], [`recover`] and [`recover_from_message`].

#![deny(missing_docs)]
#![deny(unsafe_code)]

use essential_hash::hash;
use essential_types::{Signature, Signed};
pub use secp256k1;
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1, SecretKey,
};
use serde::Serialize;

/// Sign over data with secret key using secp256k1 curve.
pub fn sign<T: Serialize>(data: T, sk: SecretKey) -> Signed<T> {
    let secp = Secp256k1::new();
    let hashed_data = hash(&data);
    let message = Message::from_digest(hashed_data);
    let (rec_id, sig) = secp
        .sign_ecdsa_recoverable(&message, &sk)
        .serialize_compact();
    let signature: Signature = Signature(sig, rec_id.to_i32().try_into().unwrap());
    Signed { data, signature }
}

/// Verify signature against data.
pub fn verify<T: Serialize>(signed: &Signed<T>) -> Result<(), secp256k1::Error> {
    let secp = Secp256k1::new();
    let hashed_data = hash(&signed.data);
    let message = Message::from_digest(hashed_data);
    let pk = recover_from_message(message, &signed.signature)?;
    secp.verify_ecdsa(
        &message,
        &secp256k1::ecdsa::Signature::from_compact(&signed.signature.0).unwrap(),
        &pk,
    )
}

/// Recover public key from `Signed.data` and `Signed.signature`
pub fn recover<T: Serialize>(signed: Signed<T>) -> Result<PublicKey, secp256k1::Error> {
    let hashed_data = hash(&signed.data);
    let message = Message::from_digest(hashed_data);
    recover_from_message(message, &signed.signature)
}

/// Recover public key from signed `secp256k1::Message` and `Signature`
pub fn recover_from_message(
    message: Message,
    signature: &Signature,
) -> Result<PublicKey, secp256k1::Error> {
    let recovery_id = RecoveryId::from_i32(i32::from(signature.1))?;
    let recoverable_signature = RecoverableSignature::from_compact(&signature.0, recovery_id)?;
    let secp = Secp256k1::new();
    let public_key = secp.recover_ecdsa(&message, &recoverable_signature)?;
    Ok(public_key)
}
