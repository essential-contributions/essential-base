//! The types of errors that might occur throughout constraint checking.

use crate::{
    asm::{self, Word},
    Stack,
};
use core::fmt;
use thiserror::Error;

/// Shorthand for a `Result` where the error type is a `CheckError`.
pub type CheckResult<T> = Result<T, CheckError>;

/// Intent checking error.
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

/// An individual operation failed during constraint checking error.
#[derive(Debug, Error)]
pub enum OpError {
    /// An error occurred during an `Access` operation.
    #[error("access operation error: {0}")]
    Access(#[from] AccessError),
    /// An error occurred during an `Alu` operation.
    #[error("ALU operation error: {0}")]
    Alu(#[from] AluError),
    /// An error occurred during a `Crypto` operation.
    #[error("crypto operation error: {0}")]
    Crypto(#[from] CryptoError),
    /// An error occurred during a `Stack` operation.
    #[error("stack operation error: {0}")]
    Stack(#[from] StackError),
    /// An error occurred while parsing an operation from bytes.
    #[error("bytecode error: {0}")]
    FromBytes(#[from] asm::FromBytesError),
}

/// Access operation error.
#[derive(Debug, Error)]
pub enum AccessError {
    /// A decision variable index was out of bounds.
    #[error("decision variable slot out of bounds")]
    DecisionSlotOutOfBounds,
    /// A solution data index provided by a transient decision variable was out of bounds.
    #[error("solution data index out of bounds")]
    SolutionDataOutOfBounds,
    /// A cycle was detected between two or more transient decision variables.
    #[error("a cycle was detected between transient decision variables")]
    TransientDecisionVariableCycle,
    /// A state slot index was out of bounds.
    #[error("state slot out of bounds")]
    StateSlotOutOfBounds,
    /// A state slot delta value was invalid. Must be `0` (pre) or `1` (post).
    #[error("invalid state slot delta: expected `0` or `1`, found {0}")]
    InvalidStateSlotDelta(Word),
}

/// ALU operation error.
#[derive(Debug, Error)]
pub enum AluError {
    /// An ALU operation overflowed a `Word` value.
    #[error("word overflow")]
    Overflow,
    /// An ALU operation underflowed a `Word` value.
    #[error("word underflow")]
    Underflow,
    /// An ALU operation (either Div or Mod) attempted to divide by zero.
    #[error("attempted to divide by zero")]
    DivideByZero,
}

/// Crypto operation error.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Failed to verify a ED25519 signature.
    #[error("failed to verify ed25519 signature: {0}")]
    Ed25519(#[from] ed25519_dalek::ed25519::Error),
}

/// Shorthand for a `Result` where the error type is a `StackError`.
pub type StackResult<T> = Result<T, StackError>;

/// Stack operation error.
#[derive(Debug, Error)]
pub enum StackError {
    /// Attempted to pop a word from an empty stack.
    #[error("attempted to pop an empty stack")]
    Empty,
    /// An index into the stack was out of bounds.
    #[error("indexed stack out of bounds")]
    IndexOutOfBounds,
    /// The stack size exceeded the size limit.
    #[error("the {}-word stack size limit was exceeded", crate::Stack::SIZE_LIMIT)]
    Overflow,
}

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
