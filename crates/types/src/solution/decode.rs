//! # Decoding
//! Decoding for solution types.

use crate::Word;

use super::Mutation;

#[cfg(test)]
mod tests;

/// Errors that can occur when decoding a predicate.
#[derive(Debug, PartialEq)]
pub enum MutationDecodeError {
    /// The words are too short for the lengths of the key or value.
    WordsTooShort,
    /// The key length is negative.
    NegativeKeyLength,
    /// The value length is negative.
    NegativeValueLength,
}

impl std::error::Error for MutationDecodeError {}

/// Decode a mutation from words.
///
/// # Layout
/// ```text
/// +-----------------+-----------------+-----------------+-----------------+
/// | key length      | key             | value length    | value           |
/// +-----------------+-----------------+-----------------+-----------------+
/// ```
pub fn decode_mutation(bytes: &[Word]) -> Result<Mutation, MutationDecodeError> {
    if bytes.len() < 2 {
        return Err(MutationDecodeError::WordsTooShort);
    }

    if bytes[0] < 0 {
        return Err(MutationDecodeError::NegativeKeyLength);
    }

    // Saturating cast
    let key_len: usize = bytes[0].try_into().unwrap_or(usize::MAX);
    let key_end = 1usize.saturating_add(key_len);
    if bytes.len() < key_end {
        return Err(MutationDecodeError::WordsTooShort);
    }
    let key = bytes[1..key_end].to_vec();

    if bytes[key_end] < 0 {
        return Err(MutationDecodeError::NegativeValueLength);
    }

    // Saturating cast
    let value_len: usize = bytes[key_end].try_into().unwrap_or(usize::MAX);

    let value_start = 2usize.saturating_add(key_len);
    let value_end = value_start.saturating_add(value_len);

    if bytes.len() < value_end {
        return Err(MutationDecodeError::WordsTooShort);
    }
    let value = bytes[value_start..value_end].to_vec();
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
        return Err(MutationDecodeError::WordsTooShort);
    }
    if bytes[0] < 0 {
        return Err(MutationDecodeError::NegativeValueLength);
    }

    // Saturating cast
    let len: usize = bytes[0].try_into().unwrap_or(usize::MAX);

    // FIXME: Do a max size check to avoid a DoS attack that allocates too much memory.
    let mut mutations = Vec::with_capacity(len);
    if len == 0 {
        return Ok(mutations);
    }
    let mut i = 1;
    while i < bytes.len() {
        let Some(b) = bytes.get(i..) else {
            return Err(MutationDecodeError::WordsTooShort);
        };
        let mutation = decode_mutation(b)?;
        let size = mutation.encode_size();
        i += size;
        mutations.push(mutation);
    }
    Ok(mutations)
}
