//! # Predicate Encoding Headers
//!
//! This module contains the encoding and decoding logic for the header of a
//! [`Predicate`] when encoding to bytes.
//! This encoding is used to hash the predicate.
//!
//! ## Encoding
//! This table describes the encoding of a `Predicate` to bytes. \
//! Bytes are encoded starting at the first row and moving down. \
//! Anything larger than a byte is encoded in big-endian. \
//! Lengths are all unsigned integers.
//!
//! | Field | Size | Description |
//! | --- | --- | --- |
//! | [`num_state_reads`] | 1 byte | [`length`] of [`state_read`] |
//! | [`num_constraints`] | 1 byte | [`length`] of [`constraints`] |
//! | encoded state read lengths | 2 bytes * [`num_state_reads`] | length of each [`state_read`] program |
//! | encoded constraint lengths | 2 bytes * [`num_constraints`] | length of each [`constraints`] program |
//! | each [`StateReadBytecode`] | each [`StateReadBytecode::len`] * [`num_state_reads`] | bytes of each [`state_read`] program |
//! | each [`ConstraintBytecode`] | each [`ConstraintBytecode::len`] * [`num_constraints`] | bytes of each [`constraints`] program |
//!
//! ## Encoding program lengths
//! This is how the lengths of the programs are encoded to bytes.
//!
//! The length of each program is encoded as a big-endian [`u16`].
//!
//! For each [`state_read`]: \
//! [`StateReadBytecode::len`] as [`u16`] as `[u8; 2]` via [u16::to_be_bytes]. \
//! Then append to [`Vec<u8>`].
//!
//! Then for each [`constraints`]: \
//! [`ConstraintBytecode::len`] as [`u16`] as `[u8; 2]` via [u16::to_be_bytes]. \
//! Then append to [`Vec<u8>`].
//!
//! ## Hashing
//! The hash of the predicate is as follows:
//! 1. Hash the bytes of the static part of the header.
//! 2. Hash the bytes of the lens part of the header.
//! 3. Hash the bytes of each [`Predicate::programs`] in the predicate (in the order of the iterator).
//!
//! [`num_state_reads`]: FixedSizeHeader::num_state_reads
//! [`num_constraints`]: FixedSizeHeader::num_constraints
//!
//! [`state_read`]: Predicate::state_read
//! [`constraints`]: Predicate::constraints
//! [`StateReadBytecode`]: super::StateReadBytecode
//! [`StateReadBytecode::len`]: super::StateReadBytecode::len
//! [`ConstraintBytecode`]: super::ConstraintBytecode
//! [`ConstraintBytecode::len`]: super::ConstraintBytecode::len
//!
//! [`length`]: Vec::len

use std::mem;

use super::Predicate;
use error::DecodeResult;

pub use error::DecodeError;
pub use error::PredicateError;

mod error;
#[cfg(test)]
mod tests;

pub(super) const LEN_SIZE_BYTES: usize = core::mem::size_of::<u16>();

/// The encoded [`Predicate`] header.
///
/// This encodes the structure of the [`Predicate`] when encoding to bytes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EncodedHeader {
    /// The fixed size part of the header.
    pub fixed_size_header: EncodedFixedSizeHeader,
    /// The dynamic lengths part of the header.
    pub lens: Vec<u8>,
}

/// The fixed size part of the header encoded to bytes.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EncodedFixedSizeHeader(pub [u8; Self::SIZE]);

/// Layout of the fixed size part of the header.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedSizeHeader {
    /// Number of state reads.
    /// This must fit in a `u8`.
    pub num_state_reads: u8,
    /// Number of constraints.
    /// This must fit in a `u8`.
    pub num_constraints: u8,
}

/// The header of a [`Predicate`] decoded.
/// This contains the indices of the [`Predicate`]'s data in a buffer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecodedHeader<'a> {
    /// The indices of the state read programs in a buffer.
    pub state_reads: &'a [u8],
    /// The indices of the constraint check programs in a buffer.
    pub constraints: &'a [u8],
}

/// Inputs to compute the size in bytes of a [`Predicate`] encoded to bytes.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct EncodedSize {
    /// The number of state read programs.
    pub num_state_reads: usize,
    /// The number of constraint check programs.
    pub num_constraints: usize,
    /// The sum of the lengths of every state read program.
    pub state_read_lens_sum: usize,
    /// The sum of the lengths of every constraint check program.
    pub constraint_lens_sum: usize,
}

/// Bounds for a [`Predicate`] to check if it's within the limits.
///
/// Limits are set out as const values on the [`Predicate`] impl.
/// For example [`Predicate::MAX_BYTES`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct PredicateBounds<S, C> {
    /// The number of state read programs.
    pub num_state_reads: usize,
    /// The number of constraint check programs.
    pub num_constraints: usize,
    /// Iterator over the lengths of each state read program.
    pub state_read_lens: S,
    /// Iterator over the lengths of each constraint check program.
    pub constraint_lens: C,
}

/// The size in bytes of an encoded [`Predicate`].
pub(super) fn encoded_size(sizes: &EncodedSize) -> usize {
    EncodedFixedSizeHeader::SIZE
        + sizes.num_state_reads * LEN_SIZE_BYTES
        + sizes.num_constraints * LEN_SIZE_BYTES
        + sizes.state_read_lens_sum
        + sizes.constraint_lens_sum
}

/// Check the bounds of a predicate.
///
/// This ensures the predicate is within the limits set by the validation rules.
///
/// Limits are set out as const values on the [`Predicate`] impl.
/// For example [`Predicate::MAX_BYTES`].
pub(super) fn check_predicate_bounds<S, C>(
    mut bounds: PredicateBounds<S, C>,
) -> Result<(), PredicateError>
where
    S: Iterator<Item = usize>,
    C: Iterator<Item = usize>,
{
    // Check the number of programs is within the limits.
    if bounds.num_state_reads > Predicate::MAX_STATE_READS {
        return Err(PredicateError::TooManyStateReads(bounds.num_state_reads));
    }
    if bounds.num_constraints > Predicate::MAX_CONSTRAINTS {
        return Err(PredicateError::TooManyConstraints(bounds.num_constraints));
    }

    // Count the total size of the state read programs.
    let mut state_read_lens_sum: usize = 0;

    // Check the size of each program.
    if let Some(err) = bounds.state_read_lens.find_map(|len| {
        state_read_lens_sum = state_read_lens_sum.saturating_add(len);
        (len > Predicate::MAX_STATE_READ_SIZE_BYTES)
            .then_some(PredicateError::StateReadTooLarge(len))
    }) {
        return Err(err);
    }

    // Count the total size of the constraint check programs.
    let mut constraint_lens_sum: usize = 0;

    // Check the size of each program.
    if let Some(err) = bounds.constraint_lens.find_map(|len| {
        constraint_lens_sum = constraint_lens_sum.saturating_add(len);
        (len > Predicate::MAX_CONSTRAINT_SIZE_BYTES)
            .then_some(PredicateError::ConstraintTooLarge(len))
    }) {
        return Err(err);
    }

    // Calculate the total encoded size of the predicate.
    let encoded_size = encoded_size(&EncodedSize {
        num_state_reads: bounds.num_state_reads,
        num_constraints: bounds.num_constraints,
        state_read_lens_sum,
        constraint_lens_sum,
    });

    // Check the total size of the encoded predicate.
    if encoded_size > Predicate::MAX_BYTES {
        return Err(PredicateError::PredicateTooLarge(encoded_size));
    }

    Ok(())
}

/// Encode the lengths of the [`Predicate::state_read`]
/// and [`Predicate::constraints`] programs into bytes.
///
/// Lengths are encoded as big-endian [`u16`] bytes.
///
/// ## Warning
/// It's the callers responsibility to ensure the lengths are within bounds
/// before calling this function.
/// Use [`check_predicate_bounds`] to ensure the lengths are within bounds.
pub(super) fn encode_program_lengths(predicate: &Predicate) -> Vec<u8> {
    let state_read_lens = predicate
        .state_read
        .iter()
        .map(Vec::as_slice)
        .flat_map(encode_bytes_length);

    let constraint_lens = predicate
        .constraints
        .iter()
        .map(Vec::as_slice)
        .flat_map(encode_bytes_length);

    let lengths_size = predicate
        .state_read
        .len()
        .saturating_add(predicate.constraints.len())
        .saturating_mul(2);

    let mut buf = Vec::with_capacity(lengths_size);
    buf.extend(state_read_lens);
    buf.extend(constraint_lens);
    buf
}

/// Encode the length of a byte slice as a big-endian u16 bytes.
///
/// ## Warning
/// It's the callers responsibility to ensure the lengths are within bounds
/// before calling this function.
/// Use [`check_predicate_bounds`] to ensure the lengths are within bounds.
fn encode_bytes_length(bytes: &[u8]) -> [u8; 2] {
    (bytes.len() as u16).to_be_bytes()
}

impl EncodedFixedSizeHeader {
    /// The size of the static part of the header in bytes.
    pub const SIZE: usize = mem::size_of::<u8>() * 2;

    /// Create a new encoded fixed size header from a [`FixedSizeHeader`].
    pub const fn new(header: FixedSizeHeader) -> Self {
        let buf = [header.num_state_reads, header.num_constraints];
        Self(buf)
    }
}

impl FixedSizeHeader {
    /// Get the range of the [`Self::num_state_reads`] value within a buffer.
    const fn num_state_reads_ix() -> core::ops::Range<usize> {
        0..core::mem::size_of::<u8>()
    }

    /// Get the range of the [`Self::num_constraints`] value within a buffer.
    const fn num_constraints_ix() -> core::ops::Range<usize> {
        let end = Self::num_state_reads_ix().end + core::mem::size_of::<u8>();
        Self::num_state_reads_ix().end..end
    }

    /// Get the [`Self::num_state_reads`] byte from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_len`].
    fn get_num_state_reads(buf: &[u8]) -> u8 {
        buf[Self::num_state_reads_ix().start]
    }

    /// Get the [`Self::num_constraints`] byte from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_len`].
    fn get_num_constraints(buf: &[u8]) -> u8 {
        buf[Self::num_constraints_ix().start]
    }

    /// Decode a fixed size header from bytes.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_len`].
    fn decode(buf: &[u8]) -> Self {
        let num_state_reads = Self::get_num_state_reads(buf);
        let num_constraints = Self::get_num_constraints(buf);
        Self {
            num_state_reads,
            num_constraints,
        }
    }

    /// Get the state read lengths bytes from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_header_len_and_program_lens`].
    fn get_state_read_lens_bytes<'a>(&self, buf: &'a [u8]) -> &'a [u8] {
        let start = Self::num_constraints_ix().end;
        let end = start + (self.num_state_reads as usize).saturating_mul(2);
        &buf[start..end]
    }

    /// Get the constraint lengths bytes from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_header_len_and_program_lens`].
    fn get_constraint_lens_bytes<'a>(&self, buf: &'a [u8]) -> &'a [u8] {
        let start =
            Self::num_constraints_ix().end + (self.num_state_reads as usize).saturating_mul(2);
        let end = start + (self.num_constraints as usize).saturating_mul(2);
        &buf[start..end]
    }

    /// Check the length is big enough to hold the static part of the header.
    ///
    /// # Errors
    /// Returns an error if the buffer is too small.
    const fn check_len(len: usize) -> DecodeResult<()> {
        if len < EncodedFixedSizeHeader::SIZE {
            return Err(DecodeError::BufferTooSmall);
        }
        Ok(())
    }

    /// Check the length is big enough to hold the full header.
    /// This includes the static part and the dynamic lengths part.
    ///
    /// # Errors
    /// Returns an error if the buffer is too small.
    fn check_header_len_and_program_lens(&self, len: usize) -> DecodeResult<()> {
        if len
            < state_len_buffer_offset(self.num_state_reads as usize, self.num_constraints as usize)
        {
            return Err(DecodeError::BufferTooSmall);
        }
        Ok(())
    }
}

impl<'a> DecodedHeader<'a> {
    /// The number of bytes this header points to.
    ///
    /// This includes the [`EncodedFixedSizeHeader::SIZE`]
    /// and the lengths of the programs as well as the programs themselves.
    pub fn bytes_len(&self) -> usize {
        let sr = self
            .state_reads
            .chunks_exact(LEN_SIZE_BYTES)
            .fold(0usize, |acc, chunk| acc.saturating_add(decode_len(chunk)));

        let c = self
            .constraints
            .chunks_exact(LEN_SIZE_BYTES)
            .fold(0usize, |acc, chunk| acc.saturating_add(decode_len(chunk)));

        let lens = self
            .state_reads
            .len()
            .saturating_add(self.constraints.len());

        EncodedFixedSizeHeader::SIZE
            .saturating_add(sr)
            .saturating_add(c)
            .saturating_add(lens)
    }

    pub(super) fn num_state_reads(&self) -> usize {
        self.state_reads.len() / LEN_SIZE_BYTES
    }

    pub(super) fn num_constraints(&self) -> usize {
        self.constraints.len() / LEN_SIZE_BYTES
    }

    /// Decode a header from bytes.
    ///
    /// # Errors
    /// Returns an error if the buffer is too small
    /// or the header is inconsistent
    /// or the predicate bounds are beyond the limits.
    pub fn decode(buf: &'a [u8]) -> DecodeResult<Self> {
        use FixedSizeHeader as Fixed;

        // Check the buffer is big enough to hold the fixed size part of the header.
        Fixed::check_len(buf.len())?;

        // Decode the fixed size part of the header.
        let fh = Fixed::decode(buf);

        // Always safe to cast u8 to usize.
        let num_state_reads = fh.num_state_reads as usize;
        let num_constraints = fh.num_constraints as usize;

        // Check the buffer is big enough to hold the full decoded header.
        // This includes the fixed size part and the dynamic lengths part.
        fh.check_header_len_and_program_lens(buf.len())?;

        let header = Self {
            state_reads: fh.get_state_read_lens_bytes(buf),
            constraints: fh.get_constraint_lens_bytes(buf),
        };

        // Check the decoded header is consistent with the fixed size part.
        header.check_consistency(&fh)?;

        // Check the bounds of the predicate.
        let bounds = PredicateBounds {
            num_state_reads,
            num_constraints,
            state_read_lens: header
                .state_reads
                .chunks_exact(LEN_SIZE_BYTES)
                .map(decode_len),
            constraint_lens: header
                .constraints
                .chunks_exact(LEN_SIZE_BYTES)
                .map(decode_len),
        };
        check_predicate_bounds(bounds)?;

        Ok(header)
    }

    /// Check the [`DecodedHeader`] is consistent with the [`FixedSizeHeader`].
    fn check_consistency(&self, header: &FixedSizeHeader) -> DecodeResult<()> {
        if self.state_reads.len() / LEN_SIZE_BYTES != header.num_state_reads as usize {
            return Err(DecodeError::IncorrectBodyLength);
        }
        if self.constraints.len() / LEN_SIZE_BYTES != header.num_constraints as usize {
            return Err(DecodeError::IncorrectBodyLength);
        }
        Ok(())
    }
}

/// Decode a length from two u8 bytes.
/// Creates a [`u16::from_be_bytes`] and casts to a [`usize`].
///
/// # Panics
/// Must be called only if the buffer is at least 2 bytes.
fn decode_len(chunk: &[u8]) -> usize {
    u16::from_be_bytes([chunk[0], chunk[1]]) as usize
}

/// The length of bytes of the header and the lengths of the programs.
pub(super) fn state_len_buffer_offset(num_state_reads: usize, num_constraints: usize) -> usize {
    EncodedFixedSizeHeader::SIZE
        + num_state_reads * LEN_SIZE_BYTES
        + num_constraints * LEN_SIZE_BYTES
}

impl From<FixedSizeHeader> for EncodedFixedSizeHeader {
    fn from(header: FixedSizeHeader) -> Self {
        Self::new(header)
    }
}

impl IntoIterator for EncodedHeader {
    type Item = u8;
    type IntoIter = core::iter::Chain<
        core::array::IntoIter<u8, { EncodedFixedSizeHeader::SIZE }>,
        std::vec::IntoIter<u8>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.fixed_size_header.0.into_iter().chain(self.lens)
    }
}
