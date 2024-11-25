//! The essential VM implementation.
//!
//! ## Reading State
//!
//! The primary entrypoint for this crate is the [`Vm` type][Vm].
//!
//! The `Vm` allows for executing arbitrary [essential ASM][asm] ops.
//! The primary use-case is executing [`Program`][essential_types::predicate::Program]s
//! that make up a [`Predicate`][essential_types::predicate::Predicate]'s program graph
//! during [`Solution`][essential_types::solution::Solution] validation.
//!
//! ## Executing Ops
//!
//! There are three primary methods available for executing operations:
//!
//! - [`Vm::exec_ops`]
//! - [`Vm::exec_bytecode`]
//! - [`Vm::exec_bytecode_iter`]
//!
//! Each have slightly different performance implications, so be sure to read
//! the docs before selecting a method.
//!
//! ## Execution Future
//!
//! The `Vm::exec_*` functions all return `Future`s that not only yield on
//! async operations, but yield based on a user-specified gas limit too. See the
//! [`ExecFuture`] docs for further details on the implementation.
#![deny(missing_docs, unsafe_code)]

pub use access::{mut_keys, mut_keys_set, mut_keys_slices, Access};
pub use cached::LazyCache;
#[doc(inline)]
pub use essential_asm::{self as asm, Op};
pub use essential_types as types;
use essential_types::ContentAddress;
#[doc(inline)]
pub use future::ExecFuture;
#[doc(inline)]
pub use memory::Memory;
#[doc(inline)]
pub use op_access::OpAccess;
#[doc(inline)]
pub use repeat::Repeat;
#[doc(inline)]
pub use stack::Stack;
#[doc(inline)]
pub use state_read::StateRead;
#[doc(inline)]
pub use total_control_flow::ProgramControlFlow;
#[doc(inline)]
pub use vm::Vm;

mod access;
mod alu;
pub mod bytecode;
mod cached;
mod crypto;
pub mod error;
mod future;
mod memory;
mod op_access;
mod pred;
mod repeat;
mod sets;
mod stack;
mod state_read;
pub mod sync;
mod total_control_flow;
mod vm;

/// Shorthand for the `BytecodeMapped` type representing a mapping to/from [`Op`]s.
pub type BytecodeMapped<Bytes = Vec<u8>> = bytecode::BytecodeMapped<Op, Bytes>;
/// Shorthand for the `BytecodeMappedSlice` type for mapping [`Op`]s.
pub type BytecodeMappedSlice<'a> = bytecode::BytecodeMappedSlice<'a, Op>;
/// Shorthand for the `BytecodeMappedLazy` type for mapping [`Op`]s.
pub type BytecodeMappedLazy<I> = bytecode::BytecodeMappedLazy<Op, I>;

/// Unit used to measure gas.
pub type Gas = u64;

/// Gas limits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GasLimit {
    /// The amount that may be spent synchronously until the execution future should yield.
    pub per_yield: Gas,
    /// The total amount of gas that may be spent.
    pub total: Gas,
}

/// Distinguish between sync and async ops to ease `Future` implementation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub(crate) enum OpKind {
    /// Operations that yield immediately.
    Sync(OpSync),
    /// Operations returning a future.
    Async(OpAsync),
}

/// The set of operations that are performed asynchronously.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct OpAsync(asm::StateRead);

/// The set of operations that are performed synchronously.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum OpSync {
    /// `[asm::Access]` operations.
    Access(asm::Access),
    /// `[asm::Alu]` operations.
    Alu(asm::Alu),
    /// `[asm::TotalControlFlow]` operations.
    ControlFlow(asm::TotalControlFlow),
    /// `[asm::Crypto]` operations.
    Crypto(asm::Crypto),
    /// `[asm::Memory]` operations.
    Memory(asm::Memory),
    /// `[asm::Pred]` operations.
    Pred(asm::Pred),
    /// `[asm::Stack]` operations.
    Stack(asm::Stack),
}

/// A mapping from an operation to its gas cost.
pub trait OpGasCost {
    /// The gas cost associated with the given op.
    fn op_gas_cost(&self, op: &Op) -> Gas;
}

impl GasLimit {
    /// The default value used for the `per_yield` limit.
    // TODO: Adjust this to match recommended poll time limit on supported validator
    // hardware.
    pub const DEFAULT_PER_YIELD: Gas = 4_096;

    /// Unlimited gas limit with default gas-per-yield.
    pub const UNLIMITED: Self = Self {
        per_yield: Self::DEFAULT_PER_YIELD,
        total: Gas::MAX,
    };
}

impl From<Op> for OpKind {
    fn from(op: Op) -> Self {
        match op {
            Op::Access(op) => OpKind::Sync(OpSync::Access(op)),
            Op::Alu(op) => OpKind::Sync(OpSync::Alu(op)),
            Op::Crypto(op) => OpKind::Sync(OpSync::Crypto(op)),
            Op::Memory(op) => OpKind::Sync(OpSync::Memory(op)),
            Op::Pred(op) => OpKind::Sync(OpSync::Pred(op)),
            Op::Stack(op) => OpKind::Sync(OpSync::Stack(op)),
            Op::StateRead(op) => OpKind::Async(OpAsync(op)),
            Op::TotalControlFlow(op) => OpKind::Sync(OpSync::ControlFlow(op)),
        }
    }
}

impl<F> OpGasCost for F
where
    F: Fn(&Op) -> Gas,
{
    fn op_gas_cost(&self, op: &Op) -> Gas {
        (*self)(op)
    }
}

/// Trace the operation at the given program counter.
///
/// In the success case, also emits the resulting stack.
///
/// In the error case, emits a debug log with the error.
#[cfg(feature = "tracing")]
pub(crate) fn trace_op_res<OA, T, E>(
    oa: &mut OA,
    pc: usize,
    stack: &Stack,
    memory: &Memory,
    op_res: Result<T, E>,
) where
    OA: OpAccess,
    OA::Op: core::fmt::Debug,
    E: core::fmt::Display,
{
    let op = oa
        .op_access(pc)
        .expect("must exist as retrieved previously")
        .expect("must exist as retrieved previously");
    let pc_op = format!("0x{:02X}: {op:?}", pc);
    match op_res {
        Ok(_) => {
            tracing::trace!("{pc_op}\n  ├── {:?}\n  └── {:?}", stack, memory)
        }
        Err(ref err) => {
            tracing::trace!("{pc_op}");
            tracing::debug!("{err}");
        }
    }
}
