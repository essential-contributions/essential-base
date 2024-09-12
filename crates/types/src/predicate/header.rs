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
//! | [`Len`] of each [`StateReadBytecode`]| 2 bytes * [`num_state_reads`] | length of each [`state_read`] program |
//! | [`Len`] of each [`ConstraintBytecode`]| 2 bytes * [`num_constraints`] | length of each [`constraints`] program |
//! | each [`StateReadBytecode`] | [`length`] of each [`StateReadBytecode`] * [`num_state_reads`] | bytes of each [`state_read`] program |
//! | each [`ConstraintBytecode`] | [`length`] of each [`ConstraintBytecode`] * [`num_constraints`] | bytes of each [`constraints`] program |
//!
//! ## Hashing
//! The hash of the predicate is as follows:
//! 1. Hash the bytes of the static part of the header.
//! 2. Hash the bytes of the lens part of the header.
//! 3. Hash the bytes of each [`Predicate::programs`] in the predicate
//! (in the order of the iterator).
//!
//! [`num_state_reads`]: StaticHeaderLayout::num_state_reads
//! [`num_constraints`]: StaticHeaderLayout::num_constraints
//! [`directive_tag`]: StaticHeaderLayout::directive_tag
//! [`directive_len`]: StaticHeaderLayout::directive_len
//!
//! [`state_read`]: Predicate::state_read
//! [`constraints`]: Predicate::constraints
//! [`StateReadBytecode`]: super::StateReadBytecode
//! [`ConstraintBytecode`]: super::ConstraintBytecode
//!
//! [`length`]: Vec::len

use super::{Directive, Predicate};
use core::num::TryFromIntError;
use error::DecodeResult;

pub use error::DecodeError;
pub use error::PredicateError;

mod error;
#[cfg(test)]
mod tests;

/// The encoded [`Predicate`] header.
///
/// This encodes the structure of the [`Predicate`] when encoding to bytes.
pub struct EncodedHeader {
    /// The static part of the header.
    pub static_header: StaticHeader,
    /// The dynamic lengths part of the header.
    pub lens: Vec<u8>,
}

/// Layout of the static part of the header.
pub struct StaticHeaderLayout {
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

pub struct DecodedHeader {
    pub state_reads: Vec<core::ops::Range<usize>>,
    pub constraints: Vec<core::ops::Range<usize>>,
    pub directive: DecodedDirective,
}

pub enum DecodedDirective {
    Satisfy,
    Maximize(core::ops::Range<usize>),
    Minimize(core::ops::Range<usize>),
}

pub struct EncodedPredicate<'a> {
    pub state_reads: Vec<&'a [u8]>,
    pub constraints: Vec<&'a [u8]>,
    pub directive: EncodedDirective<'a>,
}

pub enum EncodedDirective<'a> {
    Satisfy,
    Maximize(&'a [u8]),
    Minimize(&'a [u8]),
}

pub struct PredicateBytes {
    pub header: DecodedHeader,
    pub bytes: Vec<u8>,
}

/// Tag for the [`Directive`]`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DirectiveTag {
    /// All constraints must be satisfied.
    #[default]
    Satisfy,
    /// Maximize the objective value.
    Maximize,
    /// Minimize the objective value.
    Minimize,
}

pub struct StaticHeader(pub [u8; Self::SIZE]);

pub struct Len<T>(T);

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EncodedSize {
    pub num_state_reads: usize,
    pub num_constraints: usize,
    pub state_read_lens_sum: usize,
    pub constraint_lens_sum: usize,
    pub directive_size: usize,
}
pub struct PredicateBounds<S, C> {
    pub num_state_reads: usize,
    pub num_constraints: usize,
    pub state_read_lens: S,
    pub constraint_lens: C,
    pub directive_size: usize,
}

pub fn encoded_size(sizes: &EncodedSize) -> usize {
    StaticHeader::SIZE
        + sizes.num_state_reads * core::mem::size_of::<u16>()
        + sizes.num_constraints * core::mem::size_of::<u16>()
        + sizes.state_read_lens_sum
        + sizes.constraint_lens_sum
        + sizes.directive_size
}

/// Check the bounds of a predicate.
///
/// This ensures the predicate is within the limits set by the validation rules.
pub fn check_predicate_bounds<S, C>(mut bounds: PredicateBounds<S, C>) -> Result<(), PredicateError>
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

    if encoded_size > Predicate::MAX_PREDICATE_BYTES {
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
        let static_header = StaticHeader::try_from(predicate)?;
        let lens = encode_program_lengths(predicate);
        Ok(Self {
            static_header,
            lens,
        })
    }
}

impl TryFrom<&Predicate> for StaticHeader {
    type Error = PredicateError;

    fn try_from(predicate: &Predicate) -> Result<Self, Self::Error> {
        Ok(StaticHeader::from(StaticHeaderLayout::try_from(predicate)?))
    }
}

impl TryFrom<&Predicate> for StaticHeaderLayout {
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

impl DecodedHeader {
    pub fn decode(buf: &[u8]) -> DecodeResult<Self> {
        use StaticHeaderLayout as S;
        S::check_len(buf.len())?;

        let sh = S::decode(buf)?;

        // Following casts are safe because we have checked the buffer length.

        let num_state_reads = sh.num_state_reads as usize;
        let num_constraints = sh.num_constraints as usize;

        let mut header = DecodedHeader {
            state_reads: Vec::with_capacity(num_state_reads),
            constraints: Vec::with_capacity(num_constraints),
            directive: DecodedDirective::Satisfy,
        };

        sh.check_full_buf_len(buf)?;

        let mut last = sh.full_len();
        let state_read_lens = sh.decode_state_read_lens(buf).map(|len| {
            let range = last..last + len;
            last += len;
            range
        });

        header.state_reads.extend(state_read_lens);

        let constraint_lens = sh.decode_constraint_lens(buf).map(|len| {
            let range = last..last + len;
            last += len;
            range
        });

        header.constraints.extend(constraint_lens);

        match sh.directive_tag {
            DirectiveTag::Satisfy => header.directive = DecodedDirective::Satisfy,
            DirectiveTag::Maximize => {
                header.directive = DecodedDirective::Maximize(last..sh.directive_len as usize);
            }
            DirectiveTag::Minimize => {
                header.directive = DecodedDirective::Minimize(last..sh.directive_len as usize);
            }
        }

        let bounds = PredicateBounds {
            num_state_reads,
            num_constraints,
            state_read_lens: header.state_reads.iter().map(ExactSizeIterator::len),
            constraint_lens: header.constraints.iter().map(ExactSizeIterator::len),
            directive_size: sh.directive_len as usize,
        };
        check_predicate_bounds(bounds)?;

        Ok(header)
    }

    pub fn decode_state_read(&self, buf: &[u8]) -> Vec<Vec<u8>> {
        self.state_reads
            .iter()
            .map(|range| buf[range.clone()].to_vec())
            .collect()
    }

    pub fn decode_constraints(&self, buf: &[u8]) -> Vec<Vec<u8>> {
        self.constraints
            .iter()
            .map(|range| buf[range.clone()].to_vec())
            .collect()
    }

    pub fn decode_directive(&self, buf: &[u8]) -> Directive {
        match &self.directive {
            DecodedDirective::Satisfy => Directive::Satisfy,
            DecodedDirective::Maximize(range) => Directive::Maximize(buf[range.clone()].to_vec()),
            DecodedDirective::Minimize(range) => Directive::Minimize(buf[range.clone()].to_vec()),
        }
    }
}

/// The layout and encoding of the lengths part of the header.
/// For each [`Predicate::state_read`]:
/// [`crate::StateReadBytecode::len`] as [`u16`] as `[u8; 2]` via [u16::to_be_bytes].
/// Then append to [`Vec<u8>`].
///
/// Then for each [`Predicate::constraints`]:
/// [`crate::ConstraintBytecode::len`] as [`u16`] as `[u8; 2]` via [u16::to_be_bytes].
/// Then append to [`Vec<u8>`].
///
/// The precise layout is:
/// ```no_run
/// let i = predicate.state_read.len() - 1;
/// let j = predicate.constraints.len() - 1;
/// [
///     state_read_len[0]_be_byte_0,
///     state_read_len[0]_be_byte_1,
///     // ...
///     state_read_len[i]_be_byte_0,
///     state_read_len[i]_be_byte_1,
///     constraint_len[0]_be_byte_0,
///     constraint_len[0]_be_byte_1,
///     // ...
///     constraint_len[j]_be_byte_0,
///     constraint_len[j]_be_byte_1,
/// ]
/// ```
///
/// At a high level we have:
/// &[`Predicate`] -> Vec<u8>
/// This maps the length of each [`Predicate::state_read`] to a byte vector.
/// Then appends the length of each [`Predicate::constraints`] to the same byte vector.
///
/// The encoding follows:
/// &[u8] -> usize -> u16 -> [u8; 2]
///
/// ## Warning
/// It's the callers responsibility to ensure the lengths are within bounds
/// before calling this function.
/// Use [`check_predicate_bounds`] to ensure the lengths are within bounds.
fn encode_program_lengths(predicate: &Predicate) -> Vec<u8> {
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

impl StaticHeader {
    /// The size of the static part of the header in bytes.
    pub const SIZE: usize = 5;

    pub const fn new(header: StaticHeaderLayout) -> Self {
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

impl StaticHeaderLayout {
    pub const fn num_state_reads_ix() -> core::ops::Range<usize> {
        0..core::mem::size_of::<u8>()
    }

    pub const fn num_constraints_ix() -> core::ops::Range<usize> {
        let end = Self::num_state_reads_ix().end + core::mem::size_of::<u8>();
        Self::num_state_reads_ix().end..end
    }

    pub const fn directive_tag_ix() -> core::ops::Range<usize> {
        let end = Self::num_constraints_ix().end + core::mem::size_of::<u8>();
        Self::num_constraints_ix().end..end
    }

    pub const fn directive_len_ix() -> core::ops::Range<usize> {
        let end = Self::directive_tag_ix().end + core::mem::size_of::<u16>();
        Self::directive_tag_ix().end..end
    }

    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`StaticHeaderLayout::check_buf_len`].
    pub fn get_num_state_reads(buf: &[u8]) -> u8 {
        buf[Self::num_state_reads_ix().start]
    }

    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`StaticHeaderLayout::check_buf_len`].
    pub fn get_num_constraints(buf: &[u8]) -> u8 {
        buf[Self::num_constraints_ix().start]
    }

    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`StaticHeaderLayout::check_buf_len`].
    pub fn get_directive_tag(buf: &[u8]) -> DecodeResult<DirectiveTag> {
        buf[Self::directive_tag_ix().start]
            .try_into()
            .map_err(|_| DecodeError::MissingDirectiveTag)
    }

    /// Panics if the buffer is too small.
    /// Lengths must be checked before calling this method.
    /// Check with [`StaticHeaderLayout::check_buf_len`].
    pub fn get_directive_len(buf: &[u8]) -> u16 {
        let l = &buf[Self::directive_len_ix()];
        u16::from_be_bytes([l[0], l[1]])
    }

    pub fn get_state_read_lens<'a>(&self, buf: &'a [u8]) -> &'a [u8] {
        &buf[Self::directive_len_ix().end..self.num_state_reads as usize]
    }

    pub fn get_constraint_lens<'a>(&self, buf: &'a [u8]) -> &'a [u8] {
        let start = Self::directive_len_ix().end + self.num_state_reads as usize;
        &buf[start..self.num_constraints as usize]
    }

    pub const fn check_len(len: usize) -> DecodeResult<()> {
        if len < StaticHeader::SIZE {
            return Err(DecodeError::BufferTooSmall);
        }
        Ok(())
    }

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

    pub fn decode_state_read_lens<'h, 'b: 'h>(
        &'h self,
        buf: &'b [u8],
    ) -> impl Iterator<Item = usize> + 'h {
        self.get_state_read_lens(buf)
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]) as usize)
    }

    pub fn decode_constraint_lens<'h, 'b: 'h>(
        &'h self,
        buf: &'b [u8],
    ) -> impl Iterator<Item = usize> + 'h {
        self.get_constraint_lens(buf)
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]) as usize)
    }

    pub fn full_len(&self) -> usize {
        StaticHeader::SIZE
            + self.num_state_reads as usize * core::mem::size_of::<u16>()
            + self.num_constraints as usize * core::mem::size_of::<u16>()
    }

    /// Check the length is big enough to hold the full header.
    /// This includes the static part and the dynamic lengths part.
    pub fn check_full_len(&self, len: usize) -> DecodeResult<()> {
        if len < self.full_len() {
            return Err(DecodeError::BufferTooSmall);
        }
        Ok(())
    }
    /// Check the length of the buffer is big enough to hold the header.
    pub fn check_full_buf_len(&self, buf: &[u8]) -> DecodeResult<()> {
        self.check_full_len(buf.len())
    }
}

impl From<StaticHeaderLayout> for StaticHeader {
    fn from(header: StaticHeaderLayout) -> Self {
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
        core::array::IntoIter<u8, { StaticHeader::SIZE }>,
        std::vec::IntoIter<u8>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.static_header.0.into_iter().chain(self.lens)
    }
}
