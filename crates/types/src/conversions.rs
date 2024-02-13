//! Helper functions for converting between byte and word representations.

use crate::{IntentAddress, Word};

fn pack_bytes(result: &[u8]) -> Word {
    let mut out: Word = 0;
    for (i, byte) in result.iter().rev().enumerate() {
        out |= (*byte as Word) << (i * 8);
    }
    out
}

fn unpack_bytes(word: Word) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (i, byte) in out.iter_mut().rev().enumerate() {
        *byte = (word >> (i * 8)) as u8;
    }
    out
}

impl From<IntentAddress> for [Word; 4] {
    fn from(address: IntentAddress) -> Self {
        address
            .0
            .chunks_exact(8)
            .map(pack_bytes)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

impl From<[Word; 4]> for IntentAddress {
    fn from(address: [Word; 4]) -> Self {
        let mut out = [0u8; 32];
        for (a, b) in address
            .iter()
            .copied()
            .flat_map(unpack_bytes)
            .zip(out.iter_mut())
        {
            *b = a;
        }
        IntentAddress(out)
    }
}

impl From<IntentAddress> for [u8; 32] {
    fn from(address: IntentAddress) -> Self {
        address.0
    }
}
