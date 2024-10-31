//! # Predicates
//! Types needed to represent a predicate.

use crate::{serde::bytecode, ContentAddress};
pub use encode::{PredicateDecodeError, PredicateEncodeError};
use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

pub mod encode;

/// The state a program has access to.
#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[repr(u8)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum Reads {
    /// State prior to mutations.
    #[default]
    Pre = 0,
    /// State post mutations.
    Post,
}

/// A node in the graph.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Node {
    /// The start of relevant edges to this node in the edge list of the graph.
    ///
    /// Specifying [`Edge::MAX`] indicates that this node is a leaf.
    pub edge_start: Edge,
    /// The content address of the [`Program`] that this node executes.
    pub program_address: ContentAddress,
    /// Which type of state this program has access to.
    pub reads: Reads,
}

/// An edge in the graph.
pub type Edge = u16;

/// A program dependency graph.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Predicate {
    /// Programs in the graph.
    pub nodes: Vec<Node>,
    /// Dependencies between programs in the graph.
    ///
    /// Edges are directed.
    /// The edge from `A` to `B` indicates that `B` depends on `A`, i.e., `B` is a child of `A`.
    pub edges: Vec<Edge>,
}

/// A program to be executed.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Program(
    #[serde(
        serialize_with = "bytecode::serialize",
        deserialize_with = "bytecode::deserialize"
    )]
    pub Vec<u8>,
);

/// A set of programs.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Programs(pub Vec<Program>);

impl Predicate {
    /// Maximum number of nodes in a predicate.
    pub const MAX_NODES: u16 = 1000;
    /// Maximum number of edges in a predicate.
    pub const MAX_EDGES: u16 = 1000;

    /// Encode the predicate into a bytes iterator.
    pub fn encode(&self) -> Result<impl Iterator<Item = u8> + '_, PredicateEncodeError> {
        encode::encode_predicate(self)
    }

    /// The size of the encoded predicate in bytes.
    pub fn encoded_size(&self) -> usize {
        encode::predicate_encoded_size(self)
    }

    /// Decode a predicate from bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, PredicateDecodeError> {
        encode::decode_predicate(bytes)
    }

    /// The slice of edges associated with the node at the given index.
    ///
    /// Returns `None` in the case that the given node index is out of bound, or if any of the
    /// node's edges are out of bounds of the predicate's `edges` slice.
    ///
    /// If the node is a leaf, returns an empty slice.
    pub fn node_edges(&self, node_ix: usize) -> Option<&[Edge]> {
        let node = self.nodes.get(node_ix)?;
        if node.edge_start == Edge::MAX {
            return Some(&[]);
        }
        let e_start = usize::from(node.edge_start);
        let next_node_ix = node_ix.saturating_add(1);
        let e_end = match self.nodes.get(next_node_ix) {
            // If the next node isn't a leaf, use its `edge_start` as our `end`.
            Some(next) if next.edge_start != Edge::MAX => usize::from(next.edge_start),
            // If the next node is a leaf, or there is no next node, the `end` is `edges.len()`.
            Some(_) | None => self.edges.len(),
        };
        let edges = self.edges.get(e_start..e_end)?;
        Some(edges)
    }
}

impl Programs {
    /// Maximum number of programs in a set of programs.
    pub const MAX_PROGRAMS: u16 = 1000;
}

impl Program {
    /// Maximum size of a program in bytes.
    pub const MAX_SIZE: u16 = 10_000;
}
