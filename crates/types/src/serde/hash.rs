//! Custom hash serialization to better support human-readable formats.

use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The base64 encoding used for hashes (`ContentAddress`, `Signature`) in
/// human-readable serialization formats.
///
/// The goal is for this encoding to strike a nice balance between compact,
/// efficient, URL-friendly and relatively-filename-friendly.
pub use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64;

/// Serialize a fixed-size hash value (`ContentAddress`, `Signature`).
pub fn serialize<const N: usize, S>(bytes: &[u8; N], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if s.is_human_readable() {
        let string = BASE64.encode(bytes);
        string.serialize(s)
    } else {
        bytes[..].serialize(s)
    }
}

/// Deserialize a fixed-size hash value (`ContentAddress`, `Signature`).
pub fn deserialize<'de, const N: usize, D>(d: D) -> Result<[u8; N], D::Error>
where
    D: Deserializer<'de>,
    [u8; N]: TryFrom<Vec<u8>>,
{
    let bytes: Vec<u8> = if d.is_human_readable() {
        let string = String::deserialize(d)?;
        BASE64.decode(string).map_err(serde::de::Error::custom)?
    } else {
        Vec::deserialize(d)?
    };
    let len = bytes.len();
    bytes.try_into().map_err(|_err| {
        let msg = format!("failed to convert `Vec<u8>` with length {len} to `[u8; {N}]`");
        serde::de::Error::custom(msg)
    })
}
