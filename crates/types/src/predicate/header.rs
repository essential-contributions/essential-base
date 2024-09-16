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
//! | [`directive_tag`] | 1 byte | [`Directive`] variant as [`DirectiveTag`] |
//! | [`directive_len`] | 2 bytes | length of the [`Directive::Maximize`] or [`Directive::Minimize`] program or `0` |
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
//! 3. Hash the bytes of each [`Predicate::as_programs`] in the predicate
//! (in the order of the iterator).
//!
//! [`num_state_reads`]: FixedSizeHeader::num_state_reads
//! [`num_constraints`]: FixedSizeHeader::num_constraints
//! [`directive_tag`]: FixedSizeHeader::directive_tag
//! [`directive_len`]: FixedSizeHeader::directive_len
//!
//! [`state_read`]: Predicate::state_read
//! [`constraints`]: Predicate::constraints
//! [`StateReadBytecode`]: super::StateReadBytecode
//! [`StateReadBytecode::len`]: super::StateReadBytecode::len
//! [`ConstraintBytecode`]: super::ConstraintBytecode
//! [`ConstraintBytecode::len`]: super::ConstraintBytecode::len
//!
//! [`length`]: Vec::len

use super::{Directive, Predicate};
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
    /// Tag of the directive.
    /// Encoded as a `u8`.
    pub directive_tag: DirectiveTag,
    /// Length of the directive program.
    /// This must fit in a `u16`.
    ///
    /// When encoding to bytes this will be encoded as two big-endian bytes.
    pub directive_len: u16,
}

/// The header of a [`Predicate`] decoded.
/// This contains the indices of the [`Predicate`]'s data in a buffer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecodedHeader {
    /// The indices of the state read programs in a buffer.
    pub state_reads: Vec<core::ops::Range<usize>>,
    /// The indices of the constraint check programs in a buffer.
    pub constraints: Vec<core::ops::Range<usize>>,
    /// The directive and its indices in a buffer.
    pub directive: DecodedDirective,
}

/// The directive of a [`Predicate`] decoded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DecodedDirective {
    /// [`Directive::Satisfy`].
    Satisfy,
    /// [`Directive::Maximize`] with the indices to the program in a buffer.
    Maximize(core::ops::Range<usize>),
    /// [`Directive::Minimize`] with the indices to the program in a buffer.
    Minimize(core::ops::Range<usize>),
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

/// Tag for the [`Directive`]`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DirectiveTag {
    /// [`Directive::Satisfy`].
    #[default]
    Satisfy,
    /// [`Directive::Maximize`].
    Maximize,
    /// [`Directive::Minimize`].
    Minimize,
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
    /// The size of the directive program.
    pub directive_size: usize,
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
    /// The size of the directive program.
    pub directive_size: usize,
}

/// The size in bytes of an encoded [`Predicate`].
pub(super) fn encoded_size(sizes: &EncodedSize) -> usize {
    EncodedFixedSizeHeader::SIZE
        + sizes.num_state_reads * core::mem::size_of::<u16>()
        + sizes.num_constraints * core::mem::size_of::<u16>()
        + sizes.state_read_lens_sum
        + sizes.constraint_lens_sum
        + sizes.directive_size
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
    if bounds.num_state_reads > Predicate::MAX_STATE_READS {
        return Err(PredicateError::TooManyStateReads(bounds.num_state_reads));
    }
    if bounds.num_constraints > Predicate::MAX_CONSTRAINTS {
        return Err(PredicateError::TooManyConstraints(bounds.num_constraints));
    }
    let mut state_read_lens_sum: usize = 0;
    if let Some(err) = bounds.state_read_lens.find_map(|len| {
        state_read_lens_sum = state_read_lens_sum.saturating_add(len);
        (len > Predicate::MAX_STATE_READ_SIZE_BYTES)
            .then_some(PredicateError::StateReadTooLarge(len))
    }) {
        return Err(err);
    }

    let mut constraint_lens_sum: usize = 0;
    if let Some(err) = bounds.constraint_lens.find_map(|len| {
        constraint_lens_sum = constraint_lens_sum.saturating_add(len);
        (len > Predicate::MAX_CONSTRAINT_SIZE_BYTES)
            .then_some(PredicateError::ConstraintTooLarge(len))
    }) {
        return Err(err);
    }

    let encoded_size = encoded_size(&EncodedSize {
        num_state_reads: bounds.num_state_reads,
        num_constraints: bounds.num_constraints,
        state_read_lens_sum,
        constraint_lens_sum,
        directive_size: bounds.directive_size,
    });

    if encoded_size > Predicate::MAX_BYTES {
        return Err(PredicateError::PredicateTooLarge(encoded_size));
    }
    // Check the directive size.
    if bounds.directive_size > Predicate::MAX_DIRECTIVE_SIZE_BYTES {
        return Err(PredicateError::DirectiveTooLarge(bounds.directive_size));
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

        // All following casts are safe because we have checked the bounds.
        let directive_len = match &predicate.directive {
            Directive::Satisfy => 0,
            Directive::Maximize(program) | Directive::Minimize(program) => program.len() as u16,
        };
        Ok(Self {
            num_state_reads: predicate.state_read.len() as u8,
            num_constraints: predicate.constraints.len() as u8,
            directive_tag: DirectiveTag::from(&predicate.directive),
            directive_len,
        })
    }
}

/// TODO: FIX THESE DOCS
///
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
    pub const SIZE: usize = 5;

    /// Create a new encoded fixed size header from a [`FixedSizeHeader`].
    pub const fn new(header: FixedSizeHeader) -> Self {
        let [directive_len_0, directive_len_1] = header.directive_len.to_be_bytes();
        let buf = [
            header.num_state_reads,
            header.num_constraints,
            header.directive_tag as u8,
            directive_len_0,
            directive_len_1,
        ];
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

    /// Get the [`Self::directive_tag`] indices for within a buffer.
    pub const fn directive_tag_ix() -> core::ops::Range<usize> {
        let end = Self::num_constraints_ix().end + core::mem::size_of::<u8>();
        Self::num_constraints_ix().end..end
    }

    /// Get the [`Self::directive_len`] indices for within a buffer.
    pub const fn directive_len_ix() -> core::ops::Range<usize> {
        let end = Self::directive_tag_ix().end + core::mem::size_of::<u16>();
        Self::directive_tag_ix().end..end
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

    /// Get the [`Self::directive_tag`] byte from a buffer.
    ///
    /// # Errors
    /// Returns an error if the byte is not a valid [`DirectiveTag`].
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_len`].
    pub fn get_directive_tag(buf: &[u8]) -> DecodeResult<DirectiveTag> {
        buf[Self::directive_tag_ix().start]
            .try_into()
            .map_err(|_| DecodeError::MissingDirectiveTag)
    }

    /// Get the [`Self::directive_len`] [`u16`] from a buffer.
    /// Uses [`u16::from_be_bytes`] to convert the bytes to a [`u16`].
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_len`].
    pub fn get_directive_len(buf: &[u8]) -> u16 {
        let l = &buf[Self::directive_len_ix()];
        u16::from_be_bytes([l[0], l[1]])
    }

    /// Decode a fixed size header from bytes.
    ///
    /// # Errors
    /// Returns an error if the byte is not a valid [`DirectiveTag`].
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_len`].
    pub fn decode(buf: &[u8]) -> DecodeResult<Self> {
        let num_state_reads = Self::get_num_state_reads(buf);
        let num_constraints = Self::get_num_constraints(buf);
        let directive_tag = Self::get_directive_tag(buf)?;
        let directive_len = Self::get_directive_len(buf);
        Ok(Self {
            num_state_reads,
            num_constraints,
            directive_tag,
            directive_len,
        })
    }

    /// Get the state read lengths bytes from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`FixedSizeHeader::check_header_len_and_program_lens`].
    pub fn get_state_read_lens_bytes<'a>(&self, buf: &'a [u8]) -> &'a [u8] {
        let start = Self::directive_len_ix().end;
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
            Self::directive_len_ix().end + (self.num_state_reads as usize).saturating_mul(2);
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
            .chunks_exact(2)
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
            .chunks_exact(2)
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
    /// or the directive tag is invalid
    /// or the predicate bounds are beyond the limits.
    pub fn decode(buf: &[u8]) -> DecodeResult<Self> {
        use FixedSizeHeader as Fixed;
        Fixed::check_len(buf.len())?;

        let fh = Fixed::decode(buf)?;

        // Following casts are safe because we have checked the buffer length.

        let num_state_reads = fh.num_state_reads as usize;
        let num_constraints = fh.num_constraints as usize;

        let mut header = DecodedHeader {
            state_reads: Vec::with_capacity(num_state_reads),
            constraints: Vec::with_capacity(num_constraints),
            directive: DecodedDirective::Satisfy,
        };

        fh.check_header_len_and_program_lens(buf.len())?;

        let mut last = fh.header_len_and_program_lens();
        let state_read_lens = fh.decode_state_read_lens(buf).map(|len| {
            let range = last..last + len;
            last += len;
            range
        });

        header.state_reads.extend(state_read_lens);

        let constraint_lens = fh.decode_constraint_lens(buf).map(|len| {
            let range = last..last + len;
            last += len;
            range
        });

        header.constraints.extend(constraint_lens);

        match fh.directive_tag {
            DirectiveTag::Satisfy => header.directive = DecodedDirective::Satisfy,
            DirectiveTag::Maximize => {
                header.directive =
                    DecodedDirective::Maximize(last..(last + fh.directive_len as usize));
            }
            DirectiveTag::Minimize => {
                header.directive =
                    DecodedDirective::Minimize(last..(last + fh.directive_len as usize));
            }
        }

        header.check_consistency(&fh)?;

        let bounds = PredicateBounds {
            num_state_reads,
            num_constraints,
            state_read_lens: header.state_reads.iter().map(ExactSizeIterator::len),
            constraint_lens: header.constraints.iter().map(ExactSizeIterator::len),
            directive_size: fh.directive_len as usize,
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
        let d = self.directive.len();
        let lens = self
            .state_reads
            .len()
            .saturating_add(self.constraints.len())
            .saturating_mul(2);
        EncodedFixedSizeHeader::SIZE
            .saturating_add(sr)
            .saturating_add(c)
            .saturating_add(d)
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

    /// Decode the directive from a buffer.
    ///
    /// # Panics
    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`Self::bytes_len`].
    pub fn decode_directive(&self, buf: &[u8]) -> Directive {
        match &self.directive {
            DecodedDirective::Satisfy => Directive::Satisfy,
            DecodedDirective::Maximize(range) => Directive::Maximize(buf[range.clone()].to_vec()),
            DecodedDirective::Minimize(range) => Directive::Minimize(buf[range.clone()].to_vec()),
        }
    }

    /// Check the [`DecodedHeader`] is consistent with the [`FixedSizeHeader`].
    pub fn check_consistency(&self, header: &FixedSizeHeader) -> DecodeResult<()> {
        if self.state_reads.len() != header.num_state_reads as usize {
            return Err(DecodeError::IncorrectBodyLength);
        }
        if self.constraints.len() != header.num_constraints as usize {
            return Err(DecodeError::IncorrectBodyLength);
        }
        match &self.directive {
            DecodedDirective::Satisfy => {
                if header.directive_tag != DirectiveTag::Satisfy {
                    return Err(DecodeError::InvalidDirectiveTag);
                }
            }
            DecodedDirective::Maximize(range) => {
                if header.directive_tag != DirectiveTag::Maximize {
                    return Err(DecodeError::InvalidDirectiveTag);
                }
                if range.len() != header.directive_len as usize {
                    return Err(DecodeError::IncorrectBodyLength);
                }
            }
            DecodedDirective::Minimize(range) => {
                if header.directive_tag != DirectiveTag::Minimize {
                    return Err(DecodeError::InvalidDirectiveTag);
                }
                if range.len() != header.directive_len as usize {
                    return Err(DecodeError::IncorrectBodyLength);
                }
            }
        }
        Ok(())
    }
}

impl DecodedDirective {
    /// Get the length of the directive program.
    pub fn len(&self) -> usize {
        match self {
            DecodedDirective::Satisfy => 0,
            DecodedDirective::Maximize(range) | DecodedDirective::Minimize(range) => range.len(),
        }
    }

    /// Check if the directive program is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<FixedSizeHeader> for EncodedFixedSizeHeader {
    fn from(header: FixedSizeHeader) -> Self {
        Self::new(header)
    }
}

impl From<&Directive> for DirectiveTag {
    fn from(d: &Directive) -> Self {
        match d {
            Directive::Satisfy => DirectiveTag::Satisfy,
            Directive::Maximize(_) => DirectiveTag::Maximize,
            Directive::Minimize(_) => DirectiveTag::Minimize,
        }
    }
}

impl TryFrom<u8> for DirectiveTag {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DirectiveTag::Satisfy),
            1 => Ok(DirectiveTag::Maximize),
            2 => Ok(DirectiveTag::Minimize),
            _ => Err(DecodeError::InvalidDirectiveTag),
        }
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
