//! The types of errors that might occur throughout state read execution.

pub use crate::constraint::error::{StackError, StackResult};
#[doc(inline)]
use crate::{
    asm::{self, Word},
    constraint, Gas,
};
use thiserror::Error;

/// Shorthand for a `Result` where the error type is a `StateReadError`.
pub type StateReadResult<T, E> = Result<T, StateReadError<E>>;

/// Shorthand for a `Result` where the error type is an `OpError`.
pub type OpResult<T, E> = Result<T, OpError<E>>;

/// Shorthand for a `Result` where the error type is an `OpSyncError`.
pub type OpSyncResult<T> = Result<T, OpSyncError>;

/// Shorthand for a `Result` where the error type is an `OpAsyncError`.
pub type OpAsyncResult<T, E> = Result<T, OpAsyncError<E>>;

/// Shorthand for a `Result` where the error type is a `MemoryError`.
pub type MemoryResult<T> = Result<T, StateSlotsError>;

/// State read execution failure.
#[derive(Debug, Error)]
pub enum StateReadError<E> {
    /// The operation at the specified index failed.
    #[error("operation at index {0} failed: {1}")]
    Op(usize, OpError<E>),
    /// The program counter is out of range.
    #[error("program counter {0} out of range (note: programs must end with `Halt`)")]
    PcOutOfRange(usize),
}

/// An individual operation failed during state read execution.
#[derive(Debug, Error)]
pub enum OpError<E> {
    /// A synchronous operation failed.
    #[error("synchronous operation failed: {0}")]
    Sync(#[from] OpSyncError),
    /// An asynchronous operation failed.
    #[error("asynchronous operation failed: {0}")]
    Async(#[from] OpAsyncError<E>),
    /// An error occurred while parsing an operation from bytes.
    #[error("bytecode error: {0}")]
    FromBytes(#[from] asm::FromBytesError),
    /// The total gas limit was exceeded.
    #[error("{0}")]
    OutOfGas(#[from] OutOfGasError),
}

/// The gas cost of performing an operation would exceed the gas limit.
#[derive(Debug, Error)]
#[error(
    "operation cost would exceed gas limit\n  \
    spent: {spent} gas\n  \
    op cost: {op_gas} gas\n  \
    limit: {limit} gas"
)]
pub struct OutOfGasError {
    /// Total spent prior to the operation that would exceed the limit.
    pub spent: Gas,
    /// The gas required for the operation that failed.
    pub op_gas: Gas,
    /// The total gas limit that would be exceeded.
    pub limit: Gas,
}

/// A synchronous operation failed.
#[derive(Debug, Error)]
pub enum OpSyncError {
    /// An error occurred during a `Constraint` operation.
    #[error("constraint operation error: {0}")]
    Constraint(#[from] constraint::error::OpError),
    /// An error occurred during a `ControlFlow` operation.
    #[error("control flow operation error: {0}")]
    ControlFlow(#[from] ControlFlowError),
    /// An error occurred during a `StateSlots` operation.
    #[error("memory operation error: {0}")]
    StateSlots(#[from] StateSlotsError),
    /// The next program counter would overflow.
    #[error("the next program counter would overflow")]
    PcOverflow,
}

/// A synchronous operation failed.
#[derive(Debug, Error)]
pub enum OpAsyncError<E> {
    /// An error occurred during a `StateRead` operation.
    #[error("state read operation error: {0}")]
    StateRead(E),
    /// A `Memory` access related error occurred.
    #[error("memory error: {0}")]
    Memory(#[from] StateSlotsError),
    /// An error occurred during a `Stack` operation.
    #[error("stack operation error: {0}")]
    Stack(#[from] StackError),
    /// The next program counter would overflow.
    #[error("the next program counter would overflow")]
    PcOverflow,
}

/// Errors occuring during `ControlFlow` operation.
#[derive(Debug, Error)]
pub enum ControlFlowError {
    /// A `JumpIf` operation encountered an invalid condition.
    ///
    /// Condition values must be 0 (false) or 1 (true).
    #[error("invalid condition value {0}, expected 0 (false) or 1 (true)")]
    InvalidJumpIfCondition(Word),
}

/// Errors occuring during `StateSlots` operation.
#[derive(Debug, Error)]
pub enum StateSlotsError {
    /// Attempted to access a memory index that was out of bounds.
    #[error("index out of bounds")]
    IndexOutOfBounds,
    /// An operation would have caused memory to overflow.
    #[error("operation would cause memory to overflow")]
    Overflow,
}

impl<E> From<core::convert::Infallible> for OpError<E> {
    fn from(err: core::convert::Infallible) -> Self {
        match err {}
    }
}

impl From<StackError> for OpSyncError {
    fn from(err: StackError) -> Self {
        OpSyncError::Constraint(err.into())
    }
}
