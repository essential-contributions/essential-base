//! Crypto operation implementations.

use crate::{
    asm::Word,
    error::{CryptoError, OpError},
    OpResult, Stack,
};
use essential_types::convert::{
    bytes_from_word, u8_32_from_word_4, u8_64_from_word_8, word_4_from_u8_32, word_from_bytes,
};

#[cfg(test)]
mod tests;

/// `Crypto::Sha256` implementation.
pub(crate) fn sha256(stack: &mut Stack) -> OpResult<()> {
    use sha2::Digest;
    let data = stack.pop_len_words::<_, Vec<_>, OpError>(|words| {
        Ok(bytes_from_words(words.iter().copied()).collect())
    })?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&data);
    let hash_bytes: [u8; 32] = hasher.finalize().into();
    let hash_words = word_4_from_u8_32(hash_bytes);
    stack.extend(hash_words)?;
    Ok(())
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
/// `Crypto::VerifyEd25519` implementation.
pub(crate) fn verify_ed25519(stack: &mut Stack) -> OpResult<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let pubkey_words = stack.pop4()?;
    let signature_words = stack.pop8()?;
    let data = stack.pop_len_words::<_, Vec<_>, OpError>(|words| {
        Ok(bytes_from_words(words.iter().copied()).collect())
    })?;
    let pubkey_bytes = u8_32_from_word_4(pubkey_words);
    let pubkey = VerifyingKey::from_bytes(&pubkey_bytes).map_err(CryptoError::Ed25519)?;
    let signature_bytes = u8_64_from_word_8(signature_words);
    let signature = Signature::from_bytes(&signature_bytes);

    #[cfg(feature = "tracing")]
    tracing::trace!("{:?}, {:?}", signature, pubkey);

    let valid = pubkey.verify(&data, &signature).is_ok();
    let word = Word::from(valid);
    stack.push(word)?;
    Ok(())
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub(crate) fn recover_secp256k1(stack: &mut Stack) -> OpResult<()> {
    use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

    // Pop the stack.
    let recover_bit = stack.pop()?;
    let signature_words = stack.pop8()?;
    let message_hash = stack.pop4()?;

    // Parse the signature data.
    let digest = u8_32_from_word_4(message_hash);
    let signature = match Signature::from_slice(&u8_64_from_word_8(signature_words)) {
        Ok(signature) => signature,
        Err(_) => {
            // Invalid signature.
            // Push zeros and return early.
            stack.extend([0; 5])?;
            return Ok(());
        }
    };
    let recovery_id = RecoveryId::new(recover_bit & 1 != 0, false);

    #[cfg(feature = "tracing")]
    tracing::trace!("{:?}", signature);

    // Recover the public key.
    match VerifyingKey::recover_from_prehash(&digest, &signature, recovery_id) {
        Ok(public_key) => {
            #[cfg(feature = "tracing")]
            tracing::trace!("{:?}", public_key);
            // Serialize the public key.
            // Note the public key is 33 bytes long.
            let encoded_point = public_key.to_encoded_point(true);
            let public_key_bytes = encoded_point.as_bytes();
            let public_key_word = word_4_from_u8_32(public_key_bytes[..32].try_into().unwrap());
            let mut end_word = [0u8; 8];
            end_word[7] = public_key_bytes[32];
            let end_word = word_from_bytes(end_word);

            // Push the public key.
            stack.extend(public_key_word)?;
            stack.push(end_word)?;
        }
        // If the public key could not be recovered, push zeros.
        Err(_) => stack.extend([0; 5])?,
    }

    Ok(())
}

fn bytes_from_words(words: impl IntoIterator<Item = Word>) -> impl Iterator<Item = u8> {
    words.into_iter().flat_map(bytes_from_word)
}
