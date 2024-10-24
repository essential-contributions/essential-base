//! # Predicates
//! Types needed to represent a predicate.

use crate::{serde::bytecode, ConstraintBytecode, ContentAddress, StateReadBytecode};
use header::{check_predicate_bounds, encoded_size, EncodedSize, PredicateBounds, PredicateError};
use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

#[cfg(test)]
mod tests;

pub mod encode_predicate;
pub mod header;

/// The state a program has access to.
#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum Reads {
    /// State prior to mutations.
    #[default]
    Pre = 0,
    /// State post mutations.
    Post,
}

/// A node in the graph.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// An individual predicate to be solved.
pub struct OldPredicate {
    /// The programs that read state.
    #[serde(
        serialize_with = "bytecode::serialize_vec",
        deserialize_with = "bytecode::deserialize_vec"
    )]
    pub state_read: Vec<StateReadBytecode>,
    /// The programs that check constraints.
    #[serde(
        serialize_with = "bytecode::serialize_vec",
        deserialize_with = "bytecode::deserialize_vec"
    )]
    pub constraints: Vec<ConstraintBytecode>,
}

impl Predicate {
    /// Maximum number of nodes in a predicate.
    pub const MAX_NODES: u16 = 1000;
    /// Maximum number of edges in a predicate.
    pub const MAX_EDGES: u16 = 1000;
}

impl OldPredicate {
    /// Maximum number of state read programs of a predicate.
    pub const MAX_STATE_READS: usize = u8::MAX as usize;
    /// Maximum size of state read programs of a predicate in bytes.
    pub const MAX_STATE_READ_SIZE_BYTES: usize = 10_000;
    /// Maximum number of constraint check programs of a predicate.
    pub const MAX_CONSTRAINTS: usize = u8::MAX as usize;
    /// Maximum size of constraint check programs of a predicate in bytes.
    pub const MAX_CONSTRAINT_SIZE_BYTES: usize = 10_000;
    /// Maximum size of a predicate in bytes.
    pub const MAX_BYTES: usize = 1024 * 50;

    /// Iterator over the programs in the predicate.
    pub fn programs(&self) -> impl Iterator<Item = &[u8]> {
        self.state_read
            .iter()
            .chain(self.constraints.iter())
            .map(|x| x.as_slice())
    }

    /// An owning Iterator over the programs in the predicate.
    pub fn into_programs(self) -> impl Iterator<Item = Vec<u8>> {
        self.state_read.into_iter().chain(self.constraints)
    }

    /// Generate the encoding header for this predicate.
    pub fn encoded_header(&self) -> Result<header::EncodedHeader, PredicateError> {
        let static_header = self.fixed_size_header()?.into();
        let lens = header::encode_program_lengths(self);
        Ok(header::EncodedHeader {
            fixed_size_header: static_header,
            lens,
        })
    }

    /// Encode the predicate into a bytes iterator.
    pub fn encode(&self) -> Result<impl Iterator<Item = u8> + '_, PredicateError> {
        let header = self.encoded_header()?;
        Ok(header
            .into_iter()
            .chain(self.programs().flat_map(|x| x.iter().copied())))
    }

    /// The size of the encoded predicate in bytes.
    ///
    /// This uses [`usize::saturating_add`] to sum the
    /// lengths of the state read and constraint.
    /// This could lead to an incorrect size for a
    /// very large predicate. However, such a predicate
    /// would be invalid due to the size limits.
    pub fn encoded_size(&self) -> usize {
        let sizes = EncodedSize {
            num_state_reads: self.state_read.len(),
            num_constraints: self.constraints.len(),
            state_read_lens_sum: self
                .state_read
                .iter()
                .fold(0, |i, p| i.saturating_add(p.len())),
            constraint_lens_sum: self
                .constraints
                .iter()
                .fold(0, |i, p| i.saturating_add(p.len())),
        };
        encoded_size(&sizes)
    }

    /// Decode a predicate from bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, header::DecodeError> {
        // Decode the header.
        let header = header::DecodedHeader::decode(bytes)?;

        // Check the buffer is large enough to hold
        // the data that the header is pointing to.
        if bytes.len() < header.bytes_len() {
            return Err(header::DecodeError::BufferTooSmall);
        }

        let num_state_reads = header.num_state_reads();
        let num_constraints = header.num_constraints();

        let mut predicate = Self {
            state_read: Vec::with_capacity(num_state_reads),
            constraints: Vec::with_capacity(num_constraints),
        };
        let mut offset = header::state_len_buffer_offset(num_state_reads, num_constraints);

        // Decode the programs.
        predicate
            .state_read
            .extend(
                header
                    .state_reads
                    .chunks_exact(header::LEN_SIZE_BYTES)
                    .map(|chunk| {
                        let len = u16::from_be_bytes([chunk[0], chunk[1]]) as usize;
                        let start = offset;
                        offset += len;
                        bytes[start..offset].to_vec()
                    }),
            );
        predicate
            .constraints
            .extend(
                header
                    .constraints
                    .chunks_exact(header::LEN_SIZE_BYTES)
                    .map(|chunk| {
                        let len = u16::from_be_bytes([chunk[0], chunk[1]]) as usize;
                        let start = offset;
                        offset += len;
                        bytes[start..offset].to_vec()
                    }),
            );

        Ok(predicate)
    }

    /// Check the predicate is within the limits of a valid predicate.
    pub fn check_predicate_bounds(&self) -> Result<(), PredicateError> {
        let bounds = PredicateBounds {
            num_state_reads: self.state_read.len(),
            num_constraints: self.constraints.len(),
            state_read_lens: self.state_read.iter().map(|x| x.len()),
            constraint_lens: self.constraints.iter().map(|x| x.len()),
        };
        check_predicate_bounds(bounds)
    }

    fn fixed_size_header(&self) -> Result<header::FixedSizeHeader, PredicateError> {
        self.check_predicate_bounds()?;
        Ok(header::FixedSizeHeader {
            num_state_reads: self.state_read.len() as u8,
            num_constraints: self.constraints.len() as u8,
        })
    }
}
