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
}
