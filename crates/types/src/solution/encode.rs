//! # Encoding
//! Encoding for solution types.
use crate::Word;

use super::Mutation;

#[cfg(test)]
mod tests;

/// Returns the size in words of the encoded mutation.
///
/// 2 words for the key length and value length,
/// plus the length of the key and value data.
pub fn encode_mutation_size(mutation: &Mutation) -> usize {
    2 + mutation.key.len() + mutation.value.len()
}

/// Encodes a mutation into a sequence of words.
///
/// # Layout
/// ```text
/// +-----------------+-----------------+
/// | key length      | key             |
/// +-----------------+-----------------+
/// | value length    | value           |
/// +-----------------+-----------------+
/// ```
pub fn encode_mutation(mutation: &Mutation) -> impl Iterator<Item = Word> + use<'_> {
    // Saturating cast
    let key_len: Word = mutation.key.len().try_into().unwrap_or(Word::MAX);
    // Saturating cast
    let value_len: Word = mutation.value.len().try_into().unwrap_or(Word::MAX);
    std::iter::once(key_len)
        .chain(mutation.key.iter().copied())
        .chain(std::iter::once(value_len))
        .chain(mutation.value.iter().copied())
}

/// Encodes a slice of mutations into a sequence of words.
///
/// # Layout
/// ```text
/// +-----------------+-----------------+-----------------+-----------------+
/// | num mutations   | mutation 1      | mutation 2      | ...             |
/// +-----------------+-----------------+-----------------+-----------------+
/// ```
pub fn encode_mutations(mutations: &[Mutation]) -> impl Iterator<Item = Word> + use<'_> {
    // Saturating cast
    let len: Word = mutations.len().try_into().unwrap_or(Word::MAX);
    std::iter::once(len).chain(mutations.iter().flat_map(encode_mutation))
}
