//! Helpers for serializing and deserializing `Signature` types.

use serde::{Deserialize, Deserializer, Serializer};

/// Serialize a `Signature.data` as a byte array.
pub fn serialize<S>(value: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bytes(&value[..])
}

/// Deserialize a `Signature.data` from a byte array.
pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
where
    D: Deserializer<'de>,
{
    let bytes = Vec::<u8>::deserialize(deserializer)?;
    if bytes.len() != 64 {
        return Err(serde::de::Error::custom("invalid length"));
    }
    let mut result = [0; 64];
    result.copy_from_slice(&bytes);
    Ok(result)
}
