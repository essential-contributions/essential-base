//! # Decoding
//! Decoding for solution types.

use crate::Word;

use super::Mutation;

#[cfg(test)]
mod tests;

/// Errors that can occur when decoding a predicate.
#[derive(Debug, PartialEq)]
pub enum MutationDecodeError {
    /// The bytes are too short for the lengths of the key or value.
    BytesTooShort,
}

impl std::error::Error for MutationDecodeError {}

/// Decode a mutation from words.
///
/// # Layout
/// ```text
/// +-----------------+-----------------+
/// | key length      | key             |
/// +-----------------+-----------------+
/// | value length    | value           |
/// +-----------------+-----------------+
/// ```
pub fn decode_mutation(bytes: &[Word]) -> Result<Mutation, MutationDecodeError> {
    if bytes.len() < 2 {
        return Err(MutationDecodeError::BytesTooShort);
    }
    // Saturating cast
    let key_len: usize = bytes[0].try_into().unwrap_or(usize::MAX);
    if bytes.len() < 1 + key_len {
        return Err(MutationDecodeError::BytesTooShort);
    }
    let key = bytes[1..1 + key_len].to_vec();
    // Saturating cast
    let value_len: usize = bytes[1 + key_len].try_into().unwrap_or(usize::MAX);

    if bytes.len() < 2 + key_len + value_len {
        return Err(MutationDecodeError::BytesTooShort);
    }
    let value = bytes[2 + key_len..2 + key_len + value_len].to_vec();
    Ok(Mutation { key, value })
}

/// Decode a slice of mutations from words.
///
/// # Layout
/// ```text
/// +-----------------+-----------------+-----------------+-----------------+
/// | num mutations   | mutation 1      | mutation 2      | ...             |
/// +-----------------+-----------------+-----------------+-----------------+
/// ```
pub fn decode_mutations(bytes: &[Word]) -> Result<Vec<Mutation>, MutationDecodeError> {
    if bytes.is_empty() {
        return Err(MutationDecodeError::BytesTooShort);
    }
    // Saturating cast
    let len: usize = bytes[0].try_into().unwrap_or(usize::MAX);
    let mut mutations = Vec::with_capacity(len);
    if len == 0 {
        return Ok(mutations);
    }
    let mut i = 1;
    while i < bytes.len() {
        let Some(b) = bytes.get(i..) else {
            return Err(MutationDecodeError::BytesTooShort);
        };
        let mutation = decode_mutation(b)?;
        let size = mutation.encode_size();
        i += size;
        mutations.push(mutation);
    }
    Ok(mutations)
}
