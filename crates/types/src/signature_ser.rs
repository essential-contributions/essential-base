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
