use crate::IntentAddress;

fn pack_bytes(result: &[u8]) -> u64 {
    let mut out: u64 = 0;
    for (i, byte) in result.iter().rev().enumerate() {
        out |= (*byte as u64) << (i * 8);
    }
    out
}

fn unpack_bytes(word: u64) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (i, byte) in out.iter_mut().rev().enumerate() {
        *byte = (word >> (i * 8)) as u8;
    }
    out
}

impl From<IntentAddress> for [u64; 4] {
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

impl From<[u64; 4]> for IntentAddress {
    fn from(address: [u64; 4]) -> Self {
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
