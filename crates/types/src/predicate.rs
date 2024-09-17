//! # Predicates
//! Types needed to represent a predicate.

use crate::{serde::bytecode, ConstraintBytecode, StateReadBytecode};
use header::{check_predicate_bounds, encoded_size, EncodedSize, PredicateBounds, PredicateError};
use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

#[cfg(test)]
mod tests;

pub mod header;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// An individual predicate to be solved.
pub struct Predicate {
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
    pub fn as_programs(&self) -> impl Iterator<Item = &[u8]> {
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
        (self).try_into()
    }

    /// Encode the predicate into a bytes iterator.
    pub fn encode(&self) -> Result<impl Iterator<Item = u8> + '_, PredicateError> {
        let header = self.encoded_header()?;
        Ok(header
            .into_iter()
            .chain(self.as_programs().flat_map(|x| x.iter().copied())))
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
    pub fn decode<B: AsRef<[u8]>>(bytes: B) -> Result<Self, header::DecodeError> {
        let bytes = bytes.as_ref();

        // Decode the header.
        let header = header::DecodedHeader::decode(bytes)?;

        // Check the buffer is large enough to hold
        // the data that the header is pointing to.
        if bytes.len() < header.bytes_len() {
            return Err(header::DecodeError::BufferTooSmall);
        }

        // Decode the programs.
        let state_read = header.decode_state_read(bytes);
        let constraints = header.decode_constraints(bytes);
        Ok(Self {
            state_read,
            constraints,
        })
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
}
