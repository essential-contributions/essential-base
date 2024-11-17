//! The types of errors that might occur throughout constraint checking.

use crate::{
    asm::{self, Word},
    Stack,
};
use core::fmt;
use thiserror::Error;

/// Shorthand for a `Result` where the error type is a `CheckError`.
pub type CheckResult<T> = Result<T, CheckError>;

/// Predicate checking error.
#[derive(Debug, Error)]
pub enum CheckError {
    /// Errors occurred while executing one or more constraints.
    #[error("errors occurred while executing one or more constraints: {0}")]
    ConstraintErrors(#[from] ConstraintErrors),
    /// One or more constraints were unsatisfied.
    #[error("one or more constraints were unsatisfied: {0}")]
    ConstraintsUnsatisfied(#[from] ConstraintsUnsatisfied),
}

/// The index of each failed constraint alongside the error it produced.
#[derive(Debug, Error)]
pub struct ConstraintErrors(pub Vec<(usize, ConstraintError)>);

/// The index of each constraint that was not satisfied.
#[derive(Debug, Error)]
pub struct ConstraintsUnsatisfied(pub Vec<usize>);

/// Shorthand for a `Result` where the error type is a `ConstraintError`.
pub type ConstraintResult<T> = Result<T, ConstraintError>;

/// Constraint checking error.
#[derive(Debug, Error)]
pub enum ConstraintError {
    /// Evaluation should have resulted with a `0` (false) or `1` (true) at the
    /// top of the stack, but did not.
    #[error(
        "invalid constraint evaluation result\n  \
        expected: [0] (false) or [1] (true)\n  \
        found:    {0:?}"
    )]
    InvalidEvaluation(Stack),
    /// The operation at the specified index failed.
    #[error("operation at index {0} failed: {1}")]
    Op(usize, OpError),
}

/// Shorthand for a `Result` where the error type is an `OpError`.
pub type OpResult<T> = Result<T, OpError>;


impl fmt::Display for ConstraintErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("the constraints at the following indices failed: \n")?;
        for (ix, err) in &self.0 {
            f.write_str(&format!("  {ix}: {err}\n"))?;
        }
        Ok(())
    }
}

impl fmt::Display for ConstraintsUnsatisfied {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("the constraints at the following indices returned false: \n")?;
        for ix in &self.0 {
            f.write_str(&format!("  {ix}\n"))?;
        }
        Ok(())
    }
}

impl From<core::convert::Infallible> for OpError {
    fn from(err: core::convert::Infallible) -> Self {
        match err {}
    }
}
impl From<MissingAccessArgError> for OpError {
    fn from(err: MissingAccessArgError) -> Self {
        AccessError::MissingArg(err).into()
    }
}
