//! Helper functions for converting between byte, word and hex string representations.

use crate::{ContentAddress, Signature, Word};
pub use hex::FromHexError;

/// Convert a hex string slice to a `Vec<Word>`.
pub fn words_from_hex_str(str: &str) -> Result<Vec<Word>, FromHexError> {
    Ok(hex::decode(str)?
        .chunks_exact(8)
        .map(|chunk| word_from_bytes(chunk.try_into().expect("Word is always 8 bytes")))
        .collect())
}

/// Convert a slice of `Word`s to a hex string.
pub fn hex_str_from_words(words: &[Word]) -> String {
    hex::encode(
        words
            .iter()
            .flat_map(|word| bytes_from_word(*word))
            .collect::<Vec<u8>>(),
    )
}

/// Convert a `Word` to its bytes.
pub fn bytes_from_word(w: Word) -> [u8; 8] {
    w.to_be_bytes()
}

/// Convert a fixed array of bytes to a `Word`.
pub fn word_from_bytes(bytes: [u8; 8]) -> Word {
    Word::from_be_bytes(bytes)
}

/// Convert a slice of bytes to a `Word`.
///
/// Ignores any bytes beyond the first 8.
pub fn word_from_bytes_slice(bytes: &[u8]) -> Word {
    let mut word = [0; core::mem::size_of::<Word>()];
    let len = bytes.len().min(word.len());
    word[..len].copy_from_slice(&bytes[..len]);
    word_from_bytes(word)
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

/// A common conversion for 64-byte signatures.
#[rustfmt::skip]
pub fn word_8_from_u8_64(bytes: [u8; 64]) -> [Word; 8] {
    // TODO: This should be converted to use const-slicing if Rust ever provides
    // some kind of support for it.
    let [
        b0, b1, b2, b3, b4, b5, b6, b7,
        b8, b9, b10, b11, b12, b13, b14, b15,
        b16, b17, b18, b19, b20, b21, b22, b23,
        b24, b25, b26, b27, b28, b29, b30, b31,
        b32, b33, b34, b35, b36, b37, b38, b39,
        b40, b41, b42, b43, b44, b45, b46, b47,
        b48, b49, b50, b51, b52, b53, b54, b55,
        b56, b57, b58, b59, b60, b61, b62, b63,
    ] = bytes;
    [
        word_from_bytes([b0, b1, b2, b3, b4, b5, b6, b7]),
        word_from_bytes([b8, b9, b10, b11, b12, b13, b14, b15]),
        word_from_bytes([b16, b17, b18, b19, b20, b21, b22, b23]),
        word_from_bytes([b24, b25, b26, b27, b28, b29, b30, b31]),
        word_from_bytes([b32, b33, b34, b35, b36, b37, b38, b39]),
        word_from_bytes([b40, b41, b42, b43, b44, b45, b46, b47]),
        word_from_bytes([b48, b49, b50, b51, b52, b53, b54, b55]),
        word_from_bytes([b56, b57, b58, b59, b60, b61, b62, b63]),
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

/// A common conversion for 64-byte signatures.
#[rustfmt::skip]
pub fn u8_64_from_word_8(words: [Word; 8]) -> [u8; 64] {
    // TODO: This should be converted to use const array concatenation if Rust
    // ever provides some kind of support for it.
    let [w0, w1, w2, w3, w4, w5, w6, w7] = words;
    let [b0, b1, b2, b3, b4, b5, b6, b7] = bytes_from_word(w0);
    let [b8, b9, b10, b11, b12, b13, b14, b15] = bytes_from_word(w1);
    let [b16, b17, b18, b19, b20, b21, b22, b23] = bytes_from_word(w2);
    let [b24, b25, b26, b27, b28, b29, b30, b31] = bytes_from_word(w3);
    let [b32, b33, b34, b35, b36, b37, b38, b39] = bytes_from_word(w4);
    let [b40, b41, b42, b43, b44, b45, b46, b47] = bytes_from_word(w5);
    let [b48, b49, b50, b51, b52, b53, b54, b55] = bytes_from_word(w6);
    let [b56, b57, b58, b59, b60, b61, b62, b63] = bytes_from_word(w7);
    [
        b0, b1, b2, b3, b4, b5, b6, b7,
        b8, b9, b10, b11, b12, b13, b14, b15,
        b16, b17, b18, b19, b20, b21, b22, b23,
        b24, b25, b26, b27, b28, b29, b30, b31,
        b32, b33, b34, b35, b36, b37, b38, b39,
        b40, b41, b42, b43, b44, b45, b46, b47,
        b48, b49, b50, b51, b52, b53, b54, b55,
        b56, b57, b58, b59, b60, b61, b62, b63,
    ]
}

/// Convert a `Word` to its `bool` representation.
///
/// Returns `None` if the given `Word` is not `0` or `1`.
pub fn bool_from_word(word: Word) -> Option<bool> {
    match word {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

impl From<ContentAddress> for [Word; 4] {
    fn from(address: ContentAddress) -> Self {
        word_4_from_u8_32(address.0)
    }
}

impl From<ContentAddress> for [u8; 32] {
    fn from(address: ContentAddress) -> Self {
        address.0
    }
}

impl From<[Word; 4]> for ContentAddress {
    fn from(address: [Word; 4]) -> Self {
        Self(u8_32_from_word_4(address))
    }
}

impl From<[u8; 32]> for ContentAddress {
    fn from(address: [u8; 32]) -> Self {
        Self(address)
    }
}

impl From<Signature> for [u8; 65] {
    #[rustfmt::skip]
    fn from(sig: Signature) -> Self {
        let [
            b0, b1, b2, b3, b4, b5, b6, b7,
            b8, b9, b10, b11, b12, b13, b14, b15,
            b16, b17, b18, b19, b20, b21, b22, b23,
            b24, b25, b26, b27, b28, b29, b30, b31,
            b32, b33, b34, b35, b36, b37, b38, b39,
            b40, b41, b42, b43, b44, b45, b46, b47,
            b48, b49, b50, b51, b52, b53, b54, b55,
            b56, b57, b58, b59, b60, b61, b62, b63,
        ] = sig.0;
        [
            b0, b1, b2, b3, b4, b5, b6, b7,
            b8, b9, b10, b11, b12, b13, b14, b15,
            b16, b17, b18, b19, b20, b21, b22, b23,
            b24, b25, b26, b27, b28, b29, b30, b31,
            b32, b33, b34, b35, b36, b37, b38, b39,
            b40, b41, b42, b43, b44, b45, b46, b47,
            b48, b49, b50, b51, b52, b53, b54, b55,
            b56, b57, b58, b59, b60, b61, b62, b63,
            sig.1,
        ]
    }
}

impl From<[u8; 65]> for Signature {
    #[rustfmt::skip]
    fn from(bytes: [u8; 65]) -> Self {
        let [
            b0, b1, b2, b3, b4, b5, b6, b7,
            b8, b9, b10, b11, b12, b13, b14, b15,
            b16, b17, b18, b19, b20, b21, b22, b23,
            b24, b25, b26, b27, b28, b29, b30, b31,
            b32, b33, b34, b35, b36, b37, b38, b39,
            b40, b41, b42, b43, b44, b45, b46, b47,
            b48, b49, b50, b51, b52, b53, b54, b55,
            b56, b57, b58, b59, b60, b61, b62, b63,
            id,
        ] = bytes;
        let sig = [
            b0, b1, b2, b3, b4, b5, b6, b7,
            b8, b9, b10, b11, b12, b13, b14, b15,
            b16, b17, b18, b19, b20, b21, b22, b23,
            b24, b25, b26, b27, b28, b29, b30, b31,
            b32, b33, b34, b35, b36, b37, b38, b39,
            b40, b41, b42, b43, b44, b45, b46, b47,
            b48, b49, b50, b51, b52, b53, b54, b55,
            b56, b57, b58, b59, b60, b61, b62, b63,
        ];
        Signature(sig, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sample data for tests
    const WORD_SAMPLE: Word = 0x123456789ABCDEF0;
    const BYTES_SAMPLE: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
    const U8_32_SAMPLE: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D,
        0x1E, 0x1F,
    ];
    const U8_64_SAMPLE: [u8; 64] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D,
        0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C,
        0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B,
        0x3C, 0x3D, 0x3E, 0x3F,
    ];

    #[test]
    fn test_bytes_from_word() {
        assert_eq!(bytes_from_word(WORD_SAMPLE), BYTES_SAMPLE);
    }

    #[test]
    fn test_word_from_bytes() {
        assert_eq!(word_from_bytes(BYTES_SAMPLE), WORD_SAMPLE);
    }

    #[test]
    fn test_word_4_from_u8_32() {
        let expected_words = [
            Word::from_be_bytes([0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]),
            Word::from_be_bytes([0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]),
            Word::from_be_bytes([0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17]),
            Word::from_be_bytes([0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F]),
        ];
        assert_eq!(word_4_from_u8_32(U8_32_SAMPLE), expected_words);
    }

    #[test]
    fn test_u8_32_from_word_4() {
        let words = [
            Word::from_be_bytes([0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]),
            Word::from_be_bytes([0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]),
            Word::from_be_bytes([0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17]),
            Word::from_be_bytes([0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F]),
        ];
        assert_eq!(u8_32_from_word_4(words), U8_32_SAMPLE);
    }

    #[test]
    fn test_word_8_from_u8_64() {
        let expected_words = [
            Word::from_be_bytes([0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]),
            Word::from_be_bytes([0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]),
            Word::from_be_bytes([0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17]),
            Word::from_be_bytes([0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F]),
            Word::from_be_bytes([0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27]),
            Word::from_be_bytes([0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F]),
            Word::from_be_bytes([0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]),
            Word::from_be_bytes([0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F]),
        ];
        assert_eq!(word_8_from_u8_64(U8_64_SAMPLE), expected_words);
    }

    #[test]
    fn test_u8_64_from_word_8() {
        let words = [
            Word::from_be_bytes([0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]),
            Word::from_be_bytes([0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]),
            Word::from_be_bytes([0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17]),
            Word::from_be_bytes([0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F]),
            Word::from_be_bytes([0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27]),
            Word::from_be_bytes([0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F]),
            Word::from_be_bytes([0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]),
            Word::from_be_bytes([0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F]),
        ];
        assert_eq!(u8_64_from_word_8(words), U8_64_SAMPLE);
    }
}
