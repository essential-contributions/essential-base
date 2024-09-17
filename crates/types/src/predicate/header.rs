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
//! 3. Hash the bytes of each [`Predicate::as_programs`] in the predicate (in the order of the iterator).
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
pub struct DecodedHeader {
    /// The indices of the state read programs in a buffer.
    pub state_reads: Vec<core::ops::Range<usize>>,
    /// The indices of the constraint check programs in a buffer.
    pub constraints: Vec<core::ops::Range<usize>>,
}

/// Encoded [`Predicate`] bytes with the [`DecodedHeader`].
/// This allows access to the programs without decoding
/// to an actually [`Predicate`] struct.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PredicateBytes {
    /// The decoded header that points into the bytes.
    pub header: DecodedHeader,
    /// The bytes of the encoded [`Predicate`].
    pub bytes: Vec<u8>,
}

/// A reference to the encoded [`Predicate`] bytes with the [`DecodedHeader`].
/// This allows access to the programs without decoding
/// to an actually [`Predicate`] struct or cloning the bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PredicateBytesRef<'a> {
    /// The decoded header that points into the bytes.
    pub header: DecodedHeader,
    /// The bytes of the encoded [`Predicate`].
    pub bytes: &'a [u8],
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
        + sizes.num_state_reads * core::mem::size_of::<u16>()
        + sizes.num_constraints * core::mem::size_of::<u16>()
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

impl TryFrom<&Predicate> for EncodedHeader {
    type Error = PredicateError;

    /// Creates the encoded header from a [`Predicate`].
    fn try_from(predicate: &Predicate) -> Result<Self, Self::Error> {
        let static_header = EncodedFixedSizeHeader::try_from(predicate)?;
        let lens = encode_program_lengths(predicate);
        Ok(Self {
            fixed_size_header: static_header,
            lens,
        })
    }
}

impl TryFrom<&Predicate> for EncodedFixedSizeHeader {
    type Error = PredicateError;

    fn try_from(predicate: &Predicate) -> Result<Self, Self::Error> {
        Ok(EncodedFixedSizeHeader::from(FixedSizeHeader::try_from(
            predicate,
        )?))
    }
}

impl TryFrom<&Predicate> for FixedSizeHeader {
    type Error = PredicateError;

    fn try_from(predicate: &Predicate) -> Result<Self, Self::Error> {
        predicate.check_predicate_bounds()?;

        Ok(Self {
            num_state_reads: predicate.state_read.len() as u8,
            num_constraints: predicate.constraints.len() as u8,
        })
    }
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
    /// Get the [`Self::num_state_reads`] indices for within a buffer.
    pub const fn num_state_reads_ix() -> core::ops::Range<usize> {
        0..core::mem::size_of::<u8>()
    }

    /// Get the [`Self::num_constraints`] indices for within a buffer.
    pub const fn num_constraints_ix() -> core::ops::Range<usize> {
        let end = Self::num_state_reads_ix().end + core::mem::size_of::<u8>();
        Self::num_state_reads_ix().end..end
    }

    /// Get the [`Self::num_state_reads`] byte from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_len`].
    pub fn get_num_state_reads(buf: &[u8]) -> u8 {
        buf[Self::num_state_reads_ix().start]
    }

    /// Get the [`Self::num_constraints`] byte from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_len`].
    pub fn get_num_constraints(buf: &[u8]) -> u8 {
        buf[Self::num_constraints_ix().start]
    }

    /// Decode a fixed size header from bytes.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_len`].
    pub fn decode(buf: &[u8]) -> Self {
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
    pub fn get_state_read_lens_bytes<'a>(&self, buf: &'a [u8]) -> &'a [u8] {
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
    pub fn get_constraint_lens_bytes<'a>(&self, buf: &'a [u8]) -> &'a [u8] {
        let start =
            Self::num_constraints_ix().end + (self.num_state_reads as usize).saturating_mul(2);
        let end = start + (self.num_constraints as usize).saturating_mul(2);
        &buf[start..end]
    }

    /// Decode the state read lengths from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_header_len_and_program_lens`].
    pub fn decode_state_read_lens<'h, 'b: 'h>(
        &'h self,
        buf: &'b [u8],
    ) -> impl Iterator<Item = usize> + 'h {
        self.get_state_read_lens_bytes(buf)
            .chunks_exact(core::mem::size_of::<u16>())
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]) as usize)
    }

    /// Decode the constraint lengths from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_header_len_and_program_lens`].
    pub fn decode_constraint_lens<'h, 'b: 'h>(
        &'h self,
        buf: &'b [u8],
    ) -> impl Iterator<Item = usize> + 'h {
        self.get_constraint_lens_bytes(buf)
            .chunks_exact(core::mem::size_of::<u16>())
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]) as usize)
    }

    /// Check the length is big enough to hold the static part of the header.
    ///
    /// # Errors
    /// Returns an error if the buffer is too small.
    pub const fn check_len(len: usize) -> DecodeResult<()> {
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
    pub fn check_header_len_and_program_lens(&self, len: usize) -> DecodeResult<()> {
        if len < self.header_len_and_program_lens() {
            return Err(DecodeError::BufferTooSmall);
        }
        Ok(())
    }

    /// The length of bytes of the header and the lengths of the programs.
    pub fn header_len_and_program_lens(&self) -> usize {
        EncodedFixedSizeHeader::SIZE
            + self.num_state_reads as usize * core::mem::size_of::<u16>()
            + self.num_constraints as usize * core::mem::size_of::<u16>()
    }
}

impl DecodedHeader {
    /// Decode a header from bytes.
    ///
    /// # Errors
    /// Returns an error if the buffer is too small
    /// or the header is inconsistent
    /// or the predicate bounds are beyond the limits.
    pub fn decode(buf: &[u8]) -> DecodeResult<Self> {
        use FixedSizeHeader as Fixed;

        // Check the buffer is big enough to hold the fixed size part of the header.
        Fixed::check_len(buf.len())?;

        // Decode the fixed size part of the header.
        let fh = Fixed::decode(buf);

        // Always safe to cast u8 to usize.
        let num_state_reads = fh.num_state_reads as usize;
        let num_constraints = fh.num_constraints as usize;

        let mut header = DecodedHeader {
            state_reads: Vec::with_capacity(num_state_reads),
            constraints: Vec::with_capacity(num_constraints),
        };

        // Check the buffer is big enough to hold the full decoded header.
        // This includes the fixed size part and the dynamic lengths part.
        fh.check_header_len_and_program_lens(buf.len())?;

        // Get the position of the first program byte (end of the header).
        let mut last = fh.header_len_and_program_lens();

        // Decode the lengths of the programs and calculate the ranges.
        let state_read_lens = fh.decode_state_read_lens(buf).map(|len| {
            let range = last..last + len;
            last += len;
            range
        });

        header.state_reads.extend(state_read_lens);

        // Decode the lengths of the programs and calculate the ranges.
        let constraint_lens = fh.decode_constraint_lens(buf).map(|len| {
            let range = last..last + len;
            last += len;
            range
        });

        header.constraints.extend(constraint_lens);

        // Check the decoded header is consistent with the fixed size part.
        header.check_consistency(&fh)?;

        // Check the bounds of the predicate.
        let bounds = PredicateBounds {
            num_state_reads,
            num_constraints,
            state_read_lens: header.state_reads.iter().map(ExactSizeIterator::len),
            constraint_lens: header.constraints.iter().map(ExactSizeIterator::len),
        };
        check_predicate_bounds(bounds)?;

        Ok(header)
    }

    /// The number of bytes this header points to.
    ///
    /// This includes the [`EncodedFixedSizeHeader::SIZE`]
    /// and the lengths of the programs as well as the programs themselves.
    pub fn bytes_len(&self) -> usize {
        let sr = self
            .state_reads
            .iter()
            .fold(0usize, |acc, p| acc.saturating_add(p.len()));

        let c = self
            .constraints
            .iter()
            .fold(0usize, |acc, p| acc.saturating_add(p.len()));

        // Two bytes per length.
        let lens = self
            .state_reads
            .len()
            .saturating_add(self.constraints.len())
            .saturating_mul(core::mem::size_of::<u16>());

        EncodedFixedSizeHeader::SIZE
            .saturating_add(sr)
            .saturating_add(c)
            .saturating_add(lens)
    }

    /// Decode the state read programs from a buffer.
    /// This simply re-nests the programs from the flat buffer.
    /// The underlying data is not modified.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`Self::bytes_len`].
    pub fn decode_state_read(&self, buf: &[u8]) -> Vec<Vec<u8>> {
        self.state_reads
            .iter()
            .map(|range| buf[range.clone()].to_vec())
            .collect()
    }

    /// Decode the constraint check programs from a buffer.
    /// This simply re-nests the programs from the flat buffer.
    /// The underlying data is not modified.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`Self::bytes_len`].
    pub fn decode_constraints(&self, buf: &[u8]) -> Vec<Vec<u8>> {
        self.constraints
            .iter()
            .map(|range| buf[range.clone()].to_vec())
            .collect()
    }

    /// Check the [`DecodedHeader`] is consistent with the [`FixedSizeHeader`].
    pub fn check_consistency(&self, header: &FixedSizeHeader) -> DecodeResult<()> {
        if self.state_reads.len() != header.num_state_reads as usize {
            return Err(DecodeError::IncorrectBodyLength);
        }
        if self.constraints.len() != header.num_constraints as usize {
            return Err(DecodeError::IncorrectBodyLength);
        }
        Ok(())
    }
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
