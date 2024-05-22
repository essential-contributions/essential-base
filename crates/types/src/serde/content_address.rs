//! Custom [`ContentAddress`] serialization to better support human-readable formats.

use crate::ContentAddress;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for ContentAddress {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        super::hash::serialize(&self.0, s)
    }
}

impl<'de> Deserialize<'de> for ContentAddress {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(super::hash::deserialize(d)?))
    }
}
