//! Custom bytecode serialization implementation for better human-friendly format support.

use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

/// A type providing custom serialization implementations for a slice of bytecode.
struct Bytecode<T>(T);

impl Serialize for Bytecode<&[u8]> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize(self.0, s)
    }
}

impl<'de> Deserialize<'de> for Bytecode<Vec<u8>> {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(deserialize(d)?))
    }
}

/// Serialize a slice of bytecode.
pub fn serialize<S>(bytecode: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if s.is_human_readable() {
        hex::serialize(bytecode, s)
    } else {
        bytecode.serialize(s)
    }
}

/// Deserialize a slice of bytecode.
pub fn deserialize<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    if d.is_human_readable() {
        hex::deserialize(d)
    } else {
        Vec::deserialize(d)
    }
}

/// Serialize a slice of bytecode slices.
pub fn serialize_vec<S, T>(bytecode_slices: &[T], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<[u8]> + Serialize,
{
    if s.is_human_readable() {
        let mut seq = s.serialize_seq(Some(bytecode_slices.len()))?;
        for slice in bytecode_slices {
            seq.serialize_element(&Bytecode(slice.as_ref()))?;
        }
        seq.end()
    } else {
        bytecode_slices.serialize(s)
    }
}

/// Deserialize a Vec of bytecode slices.
pub fn deserialize_vec<'de, D>(d: D) -> Result<Vec<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    let slices: Vec<Bytecode<Vec<u8>>> = <_>::deserialize(d)?;
    Ok(slices.into_iter().map(|bc| bc.0).collect())
}
