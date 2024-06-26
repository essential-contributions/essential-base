//! Helpers for serializing and deserializing `Signature` types.
//!
//! Serializes the signature as a sequence of 65 bytes (64 for the signature, 1 for the ID).
//!
//! Human readable serialization formats are serialized as a 65-byte, upper hex string.

pub use super::hash::{deserialize, serialize};
use crate::Signature;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let bytes: [u8; 65] = self.clone().into();
            let string = hex::encode_upper(bytes);
            string.serialize(serializer)
        } else {
            let mut seq = serializer.serialize_seq(Some(self.0.len() + 1))?;
            for b in &self.0 {
                seq.serialize_element(b)?;
            }
            seq.serialize_element(&self.1)?;
            seq.end()
        }
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: [u8; 65] = crate::serde::hash::deserialize(d)?;
        Ok(bytes.into())
    }
}

#[cfg(feature = "schema")]
/// Custom JSON schema for `crate::Signature` due to no derive for [u8; 64].
impl schemars::JsonSchema for crate::Signature {
    fn schema_name() -> String {
        "Signature".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let sig_arr = schemars::schema::SchemaObject {
            metadata: Some(Box::new(schemars::schema::Metadata {
                description: Some("Compact signature".to_string()),
                ..Default::default()
            })),
            instance_type: Some(schemars::schema::InstanceType::Array.into()),
            array: Some(Box::new(schemars::schema::ArrayValidation {
                items: Some(gen.subschema_for::<u8>().into()),
                max_items: Some(64),
                min_items: Some(64),
                ..Default::default()
            })),
            ..Default::default()
        };
        let mut data = schemars::schema::SchemaObject {
            metadata: Some(Box::new(schemars::schema::Metadata {
                description: Some("Recoverable ECDSA signature over some data.".to_string()),
                ..Default::default()
            })),
            instance_type: Some(schemars::schema::InstanceType::Array.into()),
            ..Default::default()
        };
        let mut id = gen.subschema_for::<u8>().into_object();
        id.metadata = Some(Box::new(schemars::schema::Metadata {
            description: Some("ID used for public key recovery".to_string()),
            ..Default::default()
        }));
        let arr = data.array();
        arr.items = Some(vec![sig_arr.into(), id.into()].into());
        arr.max_items = Some(2);
        arr.min_items = Some(2);
        data.into()
    }

    fn is_referenceable() -> bool {
        true
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(Self::schema_name())
    }
}
