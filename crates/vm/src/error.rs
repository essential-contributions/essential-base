//! The types of errors that might occur throughout execution.

#[doc(inline)]
use crate::{
    asm::{self, Word},
    Gas,
};
use core::convert::Infallible;
use thiserror::Error;

/// Shorthand for a `Result` where the error type is a `ExecError`.
pub type ExecResult<T, E> = Result<T, ExecError<E>>;

/// Shorthand for a `Result` where the error type is a `EvalSyncError`.
pub type EvalResult<T, E> = Result<T, EvalError<E>>;

/// Shorthand for a `Result` where the error type is an `OpError`.
pub type OpResult<T, E = Infallible> = Result<T, OpError<E>>;

/// Execution failed at the operation at the given index.
#[derive(Debug, Error)]
#[error("operation at index {0} failed: {1}")]
pub struct ExecError<E>(pub usize, pub OpError<E>);

/// Errors that might occur during synchronous evaluation.
#[derive(Debug, Error)]
pub enum EvalError<E> {
    /// An error occurred during execution.
    #[error("{0}")]
    Exec(#[from] ExecError<E>),
    /// Evaluation should have resulted with a `0` (false) or `1` (true) at the
    /// top of the stack, but did not.
    #[error(
        "invalid constraint evaluation result\n  \
        expected: [0] (false) or [1] (true)\n  \
        found:    {0:?}"
    )]
    InvalidEvaluation(crate::Stack),
}

/// An individual operation failed during execution.
#[derive(Debug, Error)]
pub enum OpError<E = Infallible> {
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
    /// An error occurred during a `Repeat` operation.
    #[error("repeat operation error: {0}")]
    Repeat(#[from] RepeatError),
    /// An error occurred during a `TotalControlFlow` operation.
    #[error("total control flow operation error: {0}")]
    TotalControlFlow(#[from] TotalControlFlowError),
    /// An error occurred during a `Memory` operation.
    #[error("memory operation error: {0}")]
    Memory(#[from] MemoryError),
    /// An error occurred during a `ParentMemory` operation.
    #[error("parent memory operation error: {0}")]
    ParentMemory(#[from] ParentMemoryError),
    /// Pc counter overflowed.
    #[error("PC counter overflowed")]
    PcOverflow,
    /// An error occurred while decoding some data.
    #[error("decoding error: {0}")]
    Decode(#[from] DecodeError),
    /// An error occurred while encoding some data.
    #[error("encoding error: {0}")]
    Encode(#[from] EncodeError),
    /// An error occurred during a `StateRead` operation.
    #[error("state read operation error: {0}")]
    StateRead(E),
    /// An error occurred during a `Compute` operation.
    #[error("compute operation error: {0}")]
    Compute(#[from] ComputeError<E>),
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

/// A error occurred while reading state read arguments
#[derive(Debug, Error)]
pub enum StateReadArgError {
    /// A memory access related error occurred.
    #[error("memory error: {0}")]
    Memory(#[from] MemoryError),
    /// An error occurred during a `Stack` operation.
    #[error("stack operation error: {0}")]
    Stack(#[from] StackError),
}

/// Errors occuring during `TotalControlFlow` operation.
#[derive(Debug, Error)]
pub enum ControlFlowError {
    /// A `JumpIf` operation encountered an invalid condition.
    ///
    /// Condition values must be 0 (false) or 1 (true).
    #[error("invalid condition value {0}, expected 0 (false) or 1 (true)")]
    InvalidJumpIfCondition(Word),
}

/// Access operation error.
#[derive(Debug, Error)]
pub enum AccessError {
    /// A predicate data slot was out of bounds.
    #[error("predicate data slot out of bounds: {0}")]
    PredicateDataSlotIxOutOfBounds(Word),
    /// A predicate data value length was too large.
    #[error("the length of a predicate data value is too large: {0}")]
    PredicateDataValueTooLarge(usize),
    /// A predicate data index was out of bounds.
    #[error("predicate data value_ix out of bounds: {0}..{1}")]
    PredicateDataValueRangeOutOfBounds(Word, Word),
    /// A state slot index was out of bounds.
    #[error("state slot_ix out of bounds: {0}")]
    StateSlotIxOutOfBounds(Word),
    /// A state slot index was out of bounds.
    #[error("state value_ix out of bounds: {0}..{1}")]
    StateValueRangeOutOfBounds(Word, Word),
    /// A state slot delta value was invalid. Must be `0` (pre) or `1` (post).
    #[error("invalid state slot delta: expected `0` or `1`, found {0}")]
    InvalidStateSlotDelta(Word),
    /// The total length of the set of state mutations was too large.
    #[error("the total length of the set of state mutations was too large: {0}")]
    StateMutationsLengthTooLarge(usize),
    /// The state slot value was too large.
    #[error("the state slot value was too large: {0}")]
    StateValueTooLarge(usize),
    /// Key length was out of bounds.
    #[error("key length out of bounds: {0}")]
    KeyLengthOutOfBounds(Word),
    /// The access range was invalid
    #[error("invalid access range")]
    InvalidAccessRange,
    /// The length of the slots was too large large to fit in a `Word`.
    #[error("the length of the slots was too large: {0}")]
    SlotsLengthTooLarge(usize),
    /// The `which_slots` argument was invalid.
    #[error("invalid `which_slots` argument: {0}")]
    InvalidSlotType(Word),
    /// Missing argument error.
    #[error("missing `Access` argument: {0}")]
    MissingArg(#[from] MissingAccessArgError),
}

/// Missing argument error.
#[derive(Debug, Error)]
pub enum MissingAccessArgError {
    /// Missing `delta` argument for `State` operation.
    #[error("missing `delta` argument for `State` operation")]
    StateDelta,
    /// Missing `len` argument for `State` operation.
    #[error("missing `len` argument for `State` operation")]
    StateLen,
    /// Missing `value_ix` argument for `State` operation.
    #[error("missing `value_ix` argument for `State` operation")]
    StateValueIx,
    /// Missing `slot_ix` argument for `State` operation.
    #[error("missing `slot_ix` argument for `State` operation")]
    StateSlotIx,
    /// Missing `len` argument for `PredicateData` operation.
    #[error("missing `len` argument for `PredicateData` operation")]
    PredDataLen,
    /// Missing `value_ix` argument for `PredicateData` operation.
    #[error("missing `value_ix` argument for `PredicateData` operation")]
    PredDataValueIx,
    /// Missing `slot_ix` argument for `PredicateData` operation.
    #[error("missing `slot_ix` argument for `PredicateData` operation")]
    PredDataSlotIx,
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
    /// Failed to recover a SECP256k1 public key.
    #[error("failed to recover secp256k1 public key: {0}")]
    Secp256k1(#[from] secp256k1::Error),
    /// Failed to parse SECP256k1 recovery id
    #[error("failed to parse secp256k1 recovery id")]
    Secp256k1RecoveryId,
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
    /// The condition for Select or SelectRange is not `0` (false) or `1` (true).
    #[error(
        "invalid condition\n  \
        expected: [0] (false) or [1] (true)\n  \
        found:    {0}"
    )]
    InvalidCondition(Word),
    /// There was an error while calling a `len words` function.
    #[error(transparent)]
    LenWords(#[from] LenWordsError),
}

/// A `len words` error.
#[derive(Debug, Error)]
pub enum LenWordsError {
    /// A `len words` function was called with a missing length.
    #[error("missing length argument for `len words` operation")]
    MissingLength,
    /// A `len words` function was called with an invalid length.
    #[error("invalid length argument for `len words` operation: {0}")]
    InvalidLength(Word),
    /// A `len words` function was called with an out-of-bounds length.
    #[error("length argument for `len words` operation out of bounds: {0}")]
    OutOfBounds(Word),
    /// The additional length was too large for the `len words` function.
    #[error("additional length too large for `len words` operation: {0} + {1}")]
    AdditionalOutOfBounds(usize, usize),
}

/// Shorthand for a `Result` where the error type is a `RepeatError`.
pub type RepeatResult<T> = Result<T, RepeatError>;

/// Repeat operation error.
#[derive(Debug, Error)]
pub enum RepeatError {
    /// Repeat end hit with no corresponding repeat start on the stack
    #[error("attempted to repeat to empty stack")]
    Empty,
    /// Repeat counter called when stack is empty
    #[error("attempted to access repeat counter with empty stack")]
    NoCounter,
    /// Repeat counter called with an invalid count direction
    #[error("The count direction must be 0 or 1")]
    InvalidCountDirection,
    /// The repeat stack size exceeded the size limit.
    #[error("the {}-word stack size limit was exceeded", crate::Stack::SIZE_LIMIT)]
    Overflow,
}

/// Shorthand for a `Result` where the error type is a `TotalControlFlowError`.
pub type TotalControlFlowResult<T> = Result<T, TotalControlFlowError>;

/// Total control flow operation error.
#[derive(Debug, Error)]
pub enum TotalControlFlowError {
    /// Attempted to jump forward if with an invalid condition
    #[error("jump forward if requires a boolean condition")]
    InvalidJumpForwardIfCondition,
    /// Attempted to jump forward if to the same location
    #[error("jump forward if requires to jump at least one instruction")]
    JumpedToSelf,
    /// Attempted to halt if with an invalid condition
    #[error("halt if requires a boolean condition")]
    InvalidHaltIfCondition,
    /// Attempted to panic if with an invalid condition
    #[error("panic if requires a boolean condition")]
    InvalidPanicIfCondition,
    /// The `PanicIf` operation was called with a `true` argument
    #[error("program panicked with `PanicIf` operation. The stack at the time of panic: {0:?}")]
    Panic(Vec<Word>),
}

/// Shorthand for a `Result` where the error type is a `MemoryError`.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Memory operation error.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Attempted to pop a word from an empty memory.
    #[error("attempted to pop an empty memory")]
    Empty,
    /// Index into memory was out of bounds.
    #[error("indexed memory out of bounds")]
    IndexOutOfBounds,
    /// The memory size exceeded the size limit.
    #[error("the {}-word stack size limit was exceeded", crate::Memory::SIZE_LIMIT)]
    Overflow,
}

/// Parent memory operation error.
#[derive(Debug, Error)]
pub enum ParentMemoryError {
    /// Attempted to access parent memory outside of a `Compute` context.
    #[error("Attempted to access parent memory outside of a `Compute` context")]
    NoParent,
    /// A memory access error occurred.
    #[error("A memory access error occurred: {0}")]
    Memory(#[from] MemoryError),
}

/// Shorthand for a `Result` where the error type is a `ComputeError`.
pub type ComputeResult<T, E> = Result<T, ComputeError<E>>;

/// Compute operation error.
#[derive(Debug, Error)]
pub enum ComputeError<E> {
    /// Maximum compute recursion depth reached.
    #[error("Cannot exceed compute depth: {0}")]
    DepthReached(usize),
    /// An error occurred during a `Stack` operation.
    #[error("stack operation error: {0}")]
    Stack(#[from] StackError),
    /// An error occurred during execution.
    #[error("{0}")]
    Exec(#[from] Box<ExecError<E>>),
    /// Compute breadth cannot be converted to usize.
    #[error("cannot convert breadth to usize {0}")]
    BreadthNegative(Word),
}

/// Decode error.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// Decoding a set failed.
    #[error("failed to decode set: {0:?}")]
    Set(Vec<Word>),
    /// Decoding item failed because it was too large.
    #[error("item length too large: {0}")]
    ItemLengthTooLarge(usize),
}

/// Encode error.
#[derive(Debug, Error)]
pub enum EncodeError {
    /// Encoding item failed because it was too large.
    #[error("item length too large: {0}")]
    ItemLengthTooLarge(usize),
}

impl<E> From<core::convert::Infallible> for OpError<E> {
    fn from(err: core::convert::Infallible) -> Self {
        match err {}
    }
}

impl<E> From<StateReadArgError> for OpError<E> {
    fn from(err: StateReadArgError) -> Self {
        match err {
            StateReadArgError::Memory(e) => OpError::Memory(e),
            StateReadArgError::Stack(e) => OpError::Stack(e),
        }
    }
}

impl<E> OpError<E> {
    /// Convert an op error that doesn't contain a state read a generic op error.
    pub fn from_infallible(value: OpError<Infallible>) -> Self {
        match value {
            OpError::Access(access_error) => OpError::Access(access_error),
            OpError::Alu(alu_error) => OpError::Alu(alu_error),
            OpError::Crypto(crypto_error) => OpError::Crypto(crypto_error),
            OpError::Stack(stack_error) => OpError::Stack(stack_error),
            OpError::Repeat(repeat_error) => OpError::Repeat(repeat_error),
            OpError::TotalControlFlow(total_control_flow_error) => {
                OpError::TotalControlFlow(total_control_flow_error)
            }
            OpError::Memory(memory_error) => OpError::Memory(memory_error),
            OpError::ParentMemory(memory_error) => OpError::ParentMemory(memory_error),
            OpError::PcOverflow => OpError::PcOverflow,
            OpError::Decode(decode_error) => OpError::Decode(decode_error),
            OpError::Encode(encode_error) => OpError::Encode(encode_error),
            OpError::StateRead(_) => unreachable!(),
            OpError::FromBytes(from_bytes_error) => OpError::FromBytes(from_bytes_error),
            OpError::OutOfGas(out_of_gas_error) => OpError::OutOfGas(out_of_gas_error),
            OpError::Compute(_) => unreachable!(),
        }
    }
}
