use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;

use crate::check::pack_bytes;
use crate::check::unpack_bytes;
use crate::check::Directive;
use crate::data::Slots;
use crate::db::Address;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub slots: Slots,
    pub state_read: Vec<u8>,
    pub constraints: Vec<Vec<u8>>,
    pub directive: Directive,
}

impl Intent {
    pub fn address(&self) -> Address {
        let bytes = serde_json::to_vec(&self).expect("I don't think this serialization can fail");
        let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
        hasher.update(&bytes);
        let result: [u8; 32] = hasher.finalize().into();
        let mut out: Address = [0; 4];
        for (r, o) in result.chunks_exact(8).map(pack_bytes).zip(out.iter_mut()) {
            *o = r;
        }
        out
    }
}

pub fn intent_set_address<'a>(addresses: impl Iterator<Item = &'a Address>) -> Address {
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    for address in addresses {
        let mut bytes = [0u8; 32];
        for (a, b) in address
            .iter()
            .copied()
            .flat_map(unpack_bytes)
            .zip(bytes.iter_mut())
        {
            *b = a;
        }
        hasher.update(bytes);
    }
    let result: [u8; 32] = hasher.finalize().into();
    let mut out: Address = [0; 4];
    for (r, o) in result.chunks_exact(8).map(pack_bytes).zip(out.iter_mut()) {
        *o = r;
    }
    out
}