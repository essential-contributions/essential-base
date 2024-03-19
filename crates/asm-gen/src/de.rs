//! Custom deserialize implementations for enums and the `Tree` type.

use crate::{Node, StackOut, Tree};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;

impl<'de> Deserialize<'de> for Tree {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize into the expected mapping, then convert the map to an
        // ordered, sorted list based on opcode.
        let map = BTreeMap::<String, Node>::deserialize(d)?;
        let mut vec: Vec<_> = map.into_iter().collect();
        vec.sort_by_key(|(_name, node)| node.opcode());
        Ok(Self(vec))
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mapping = serde_yaml::Mapping::deserialize(d)?;
        let node = if mapping.contains_key("opcode") {
            let value = serde_yaml::Value::Mapping(mapping);
            let op = serde_yaml::from_value(value).map_err(serde::de::Error::custom)?;
            Node::Op(op)
        } else {
            let value = serde_yaml::Value::Mapping(mapping);
            let group = serde_yaml::from_value(value).map_err(serde::de::Error::custom)?;
            Node::Group(group)
        };
        Ok(node)
    }
}

impl<'de> Deserialize<'de> for StackOut {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_yaml::Value::deserialize(d)?;
        let stack_out = match &value {
            serde_yaml::Value::Sequence(_) => {
                let fixed = serde_yaml::from_value(value).map_err(serde::de::Error::custom)?;
                StackOut::Fixed(fixed)
            }
            _ => {
                let dynamic = serde_yaml::from_value(value).map_err(serde::de::Error::custom)?;
                StackOut::Dynamic(dynamic)
            }
        };
        Ok(stack_out)
    }
}
