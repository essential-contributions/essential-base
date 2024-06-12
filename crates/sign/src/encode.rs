//! Functions for encoding signatures and public keys.

use essential_types::{
    convert::{bytes_from_word, word_4_from_u8_32, word_8_from_u8_64, word_from_bytes},
    Word,
};
use secp256k1::{ecdsa::RecoverableSignature, PublicKey};

/// Encode a secp256k1 public key into 5 words.
pub fn public_key(pk: &PublicKey) -> [Word; 5] {
    let [start @ .., end] = pk.serialize();
    let start = word_4_from_u8_32(start);
    let mut end_word = [0u8; 8];
    end_word[7] = end;
    let end_word = word_from_bytes(end_word);
    let mut out = [0; 5];
    out[..4].copy_from_slice(&start);
    out[4] = end_word;
    out
}

/// Encode a secp256k1 public key into 40 bytes.
/// This is word aligned.
pub fn public_key_as_bytes(pk: &PublicKey) -> [u8; 40] {
    let mut out = [0; 40];
    let words = public_key(pk);
    for (word, out) in words.iter().zip(out.chunks_exact_mut(8)) {
        let bytes = bytes_from_word(*word);
        out.copy_from_slice(&bytes);
    }
    out
}

/// Encode a secp256k1 recoverable signature into 9 words.
pub fn signature(sig: &RecoverableSignature) -> [Word; 9] {
    let (rec_id, sig) = sig.serialize_compact();
    let rec_id = rec_id.to_i32();
    let rec_id = Word::from(rec_id);
    let sig = word_8_from_u8_64(sig);
    let mut out = [0; 9];
    out[..8].copy_from_slice(&sig);
    out[8] = rec_id;
    out
}

/// Encode a secp256k1 recoverable signature into 72 bytes.
pub fn signature_as_bytes(sig: &RecoverableSignature) -> [u8; 72] {
    let mut out = [0; 72];
    let words = signature(sig);
    for (word, out) in words.iter().zip(out.chunks_exact_mut(8)) {
        let bytes = bytes_from_word(*word);
        out.copy_from_slice(&bytes);
    }
    out
}
