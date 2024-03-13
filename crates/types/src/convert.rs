//! Helper functions for converting between byte and word representations.

use crate::{IntentAddress, Word};

/// Convert a `Word` to its bytes.
pub fn bytes_from_word(w: Word) -> [u8; 8] {
    w.to_be_bytes()
}

/// Convert a fixed array of bytes to a `Word`.
pub fn word_from_bytes(bytes: [u8; 8]) -> Word {
    Word::from_be_bytes(bytes)
}

/// Given an iterator yielding words, produces an iterator yielding bytes.
pub fn bytes_from_words(words: impl IntoIterator<Item = Word>) -> impl Iterator<Item = u8> {
    words.into_iter().flat_map(bytes_from_word)
}

/// Given an iterator yielding bytes, produces an iterator yielding words.
///
/// Note that if the given number of `bytes` is not a multiple of the `Word`
/// size, the trailing `bytes` are ignored.
pub fn words_from_bytes(bytes: impl IntoIterator<Item = u8>) -> impl Iterator<Item = Word> {
    let mut bs = bytes.into_iter();
    std::iter::from_fn(move || {
        let b0 = bs.next()?;
        let b1 = bs.next()?;
        let b2 = bs.next()?;
        let b3 = bs.next()?;
        let b4 = bs.next()?;
        let b5 = bs.next()?;
        let b6 = bs.next()?;
        let b7 = bs.next()?;
        Some(word_from_bytes([b0, b1, b2, b3, b4, b5, b6, b7]))
    })
}

/// A common conversion for 32-byte hashes and other addresses.
#[rustfmt::skip]
pub fn word_4_from_u8_32(bytes: [u8; 32]) -> [Word; 4] {
    // TODO: This should be converted to use const-slicing if Rust ever provides
    // some kind of support for it.
    let [
        b0, b1, b2, b3, b4, b5, b6, b7,
        b8, b9, b10, b11, b12, b13, b14, b15,
        b16, b17, b18, b19, b20, b21, b22, b23,
        b24, b25, b26, b27, b28, b29, b30, b31,
    ] = bytes;
    [
        word_from_bytes([b0, b1, b2, b3, b4, b5, b6, b7]),
        word_from_bytes([b8, b9, b10, b11, b12, b13, b14, b15]),
        word_from_bytes([b16, b17, b18, b19, b20, b21, b22, b23]),
        word_from_bytes([b24, b25, b26, b27, b28, b29, b30, b31]),
    ]
}

/// A common conversion for 32-byte hashes and other addresses.
#[rustfmt::skip]
pub fn u8_32_from_word_4(words: [Word; 4]) -> [u8; 32] {
    // TODO: This should be converted to use const array concatenation if Rust
    // ever provides some kind of support for it.
    let [w0, w1, w2, w3] = words;
    let [b0, b1, b2, b3, b4, b5, b6, b7] = bytes_from_word(w0);
    let [b8, b9, b10, b11, b12, b13, b14, b15] = bytes_from_word(w1);
    let [b16, b17, b18, b19, b20, b21, b22, b23] = bytes_from_word(w2);
    let [b24, b25, b26, b27, b28, b29, b30, b31] = bytes_from_word(w3);
    [
        b0, b1, b2, b3, b4, b5, b6, b7,
        b8, b9, b10, b11, b12, b13, b14, b15,
        b16, b17, b18, b19, b20, b21, b22, b23,
        b24, b25, b26, b27, b28, b29, b30, b31,
    ]
}

impl From<IntentAddress> for [Word; 4] {
    fn from(address: IntentAddress) -> Self {
        word_4_from_u8_32(address.0)
    }
}

impl From<[Word; 4]> for IntentAddress {
    fn from(address: [Word; 4]) -> Self {
        Self(u8_32_from_word_4(address))
    }
}

impl From<IntentAddress> for [u8; 32] {
    fn from(address: IntentAddress) -> Self {
        address.0
    }
}
