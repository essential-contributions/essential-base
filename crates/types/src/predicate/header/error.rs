use std::fmt::Display;

pub type DecodeResult<T> = core::result::Result<T, DecodeError>;

#[derive(Debug)]
/// Error when encoding a predicate.
pub enum PredicateError {
    /// State read too large.
    StateReadTooLarge(usize),
    /// Constraint too large.
    ConstraintTooLarge(usize),
    /// Directive too large.
    DirectiveTooLarge(usize),
    /// Too many state reads.
    TooManyStateReads(usize),
    /// Too many constraints.
    TooManyConstraints(usize),
    /// Predicate too large.
    PredicateTooLarge(usize),
}

#[derive(Debug)]
/// Error when decoding a predicate.
pub enum DecodeError {
    /// Missing number of state reads when decoding predicate header.
    MissingNumStateReads,
    /// Missing number of constraints when decoding predicate header.
    MissingNumConstraints,
    /// Missing directive tag when decoding predicate header.
    MissingDirectiveTag,
    /// Missing directive length when decoding predicate header.
    MissingDirectiveLen,
    /// Missing nested length when decoding predicate header.
    MissingNestedLen,
    /// Invalid directive tag when decoding predicate header.
    InvalidDirectiveTag,
    /// Overflow when decoding predicate.
    Overflow,
    /// Incorrect body length when decoding predicate.
    IncorrectBodyLength,
    /// Buffer too small when decoding predicate header.
    BufferTooSmall,
    /// Error with decoded predicate.
    PredicateError(PredicateError),
}

impl std::error::Error for PredicateError {}
impl std::error::Error for DecodeError {}

impl Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::MissingNumStateReads => {
                write!(
                    f,
                    "missing number of state reads when decoding predicate header"
                )
            }
            DecodeError::MissingNumConstraints => {
                write!(
                    f,
                    "missing number of constraints when decoding predicate header"
                )
            }
            DecodeError::MissingDirectiveTag => {
                write!(f, "missing directive tag when decoding predicate header")
            }
            DecodeError::MissingDirectiveLen => {
                write!(f, "missing directive length when decoding predicate header")
            }
            DecodeError::MissingNestedLen => {
                write!(f, "missing nested length when decoding predicate header")
            }
            DecodeError::InvalidDirectiveTag => {
                write!(f, "invalid directive tag when decoding predicate header")
            }
            DecodeError::Overflow => {
                write!(f, "overflow when decoding predicate")
            }
            DecodeError::IncorrectBodyLength => {
                write!(f, "incorrect body length when decoding predicate")
            }
            DecodeError::BufferTooSmall => {
                write!(f, "buffer too small when decoding predicate header")
            }
            DecodeError::PredicateError(e) => {
                write!(f, "predicate error: {}", e)
            }
        }
    }
}

impl Display for PredicateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PredicateError::StateReadTooLarge(s) => {
                write!(f, "state read too large when encoding predicate: {}", s)
            }
            PredicateError::ConstraintTooLarge(s) => {
                write!(f, "constraint too large when encoding predicate: {}", s)
            }
            PredicateError::DirectiveTooLarge(s) => {
                write!(f, "directive too large when encoding predicate: {}", s)
            }
            PredicateError::TooManyStateReads(s) => {
                write!(f, "too many state reads when encoding predicate: {}", s)
            }
            PredicateError::TooManyConstraints(s) => {
                write!(f, "too many constraints when encoding predicate: {}", s)
            }
            PredicateError::PredicateTooLarge(s) => {
                write!(f, "predicate too large when encoding predicate: {}", s)
            }
        }
    }
}

impl From<PredicateError> for DecodeError {
    fn from(e: PredicateError) -> Self {
        DecodeError::PredicateError(e)
    }
}
