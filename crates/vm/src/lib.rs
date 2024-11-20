//! The essential state read VM implementation.
//!
//! ## Reading State
//!
//! The primary entrypoint for this crate is the [`Vm` type][Vm].
//!
//! The `Vm` allows for executing operations that read state and apply any
//! necessary operations in order to form the final, expected state slot layout
//! within the VM's [`Memory`]. The `Vm`'s memory can be accessed directly
//! from the `Vm`, or the `Vm` can be consumed and state slots returned with
//! [`Vm::into_state_slots`].
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
pub use error::{ConstraintResult, OpAsyncResult, OpResult, OpSyncResult, StateReadResult};
use error::{OpSyncError, StateReadError};
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
pub mod constraint;
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
mod total_control_flow;
mod vm;

/// Unit used to measure gas.
pub type Gas = u64;

/// Shorthand for the `BytecodeMapped` type representing a mapping to/from state read [`Op`]s.
pub type BytecodeMapped<Bytes = Vec<u8>> = bytecode::BytecodeMapped<Op, Bytes>;
/// Shorthand for the `BytecodeMappedSlice` type for mapping [`Op`]s.
pub type BytecodeMappedSlice<'a> = bytecode::BytecodeMappedSlice<'a, Op>;
/// Shorthand for the `BytecodeMappedLazy` type for mapping [`Op`]s.
pub type BytecodeMappedLazy<I> = bytecode::BytecodeMappedLazy<Op, I>;

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
    Sync(asm::Constraint),
    /// Operations returning a future.
    Async(OpAsync),
}

/// The contract of operations that are performed asynchronously.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub(crate) enum OpAsync {
    /// Read a range of values from state starting at the key.
    StateReadKeyRange,
    /// Read a range of values from external state starting at the key.
    StateReadKeyRangeExt,
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
            Op::Constraint(op) => OpKind::Sync(op),
            Op::KeyRange => OpKind::Async(OpAsync::StateReadKeyRange),
            Op::KeyRangeExtern => OpKind::Async(OpAsync::StateReadKeyRangeExt),
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

/// Step forward the VM by a single synchronous operation.
///
/// Returns a `Some(usize)` representing the new program counter resulting from
/// this step, or `None` in the case that execution has halted.
pub(crate) fn step_op_sync(
    op: asm::Constraint,
    access: Access,
    vm: &mut Vm,
) -> OpSyncResult<Option<usize>> {
    let Vm {
        stack,
        repeat,
        pc,
        memory,
        cache,
        ..
    } = vm;
    match constraint::step_op(access, op, stack, memory, *pc, repeat, cache)? {
        Some(ProgramControlFlow::Pc(pc)) => return Ok(Some(pc)),
        Some(ProgramControlFlow::Halt) => return Ok(None),
        None => (),
    }
    // Every operation besides control flow steps forward program counter by 1.
    let new_pc = vm.pc.checked_add(1).ok_or(OpSyncError::PcOverflow)?;
    Ok(Some(new_pc))
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
)
where
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
