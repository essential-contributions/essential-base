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

use constraint::{ProgramControlFlow, Repeat};
#[doc(inline)]
pub use error::{OpAsyncResult, OpResult, OpSyncResult, StateReadResult};
use error::{OpError, OpSyncError, StateReadError};
use essential_constraint_vm::LazyCache;
#[doc(inline)]
pub use essential_constraint_vm::{self as constraint, Access, OpAccess, Stack};
#[doc(inline)]
pub use essential_state_asm as asm;
use essential_state_asm::Op;
pub use essential_types as types;
use essential_types::ContentAddress;
#[doc(inline)]
pub use future::ExecFuture;
pub use state_read::StateRead;

pub mod error;
mod future;
mod state_read;

/// The operation execution state of the State Read VM.
#[derive(Debug, Default, PartialEq)]
pub struct Vm {
    /// The program counter, i.e. index of the current operation within the program.
    pub pc: usize,
    /// The stack machine.
    pub stack: Stack,
    /// The memory for temporary storage of words.
    pub memory: essential_constraint_vm::Memory,
    /// The repeat stack.
    pub repeat: Repeat,
    /// Lazily cached data for the VM.
    pub cache: LazyCache,
}

/// Unit used to measure gas.
pub type Gas = u64;

/// Shorthand for the `BytecodeMapped` type representing a mapping to/from state read [`Op`]s.
pub type BytecodeMapped<Bytes = Vec<u8>> = constraint::BytecodeMapped<Op, Bytes>;
/// Shorthand for the `BytecodeMappedSlice` type for mapping [`Op`]s.
pub type BytecodeMappedSlice<'a> = constraint::BytecodeMappedSlice<'a, Op>;
/// Shorthand for the `BytecodeMappedLazy` type for mapping [`Op`]s.
pub type BytecodeMappedLazy<I> = constraint::BytecodeMappedLazy<Op, I>;

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

impl Vm {
    /// Execute the given operations from the current state of the VM.
    ///
    /// Upon reaching a `Halt` operation or reaching the end of the operation
    /// sequence, returns the gas spent and the `Vm` will be left in the
    /// resulting state.
    ///
    /// This is a wrapper around [`Vm::exec`] that expects operation access in
    /// the form of a `&[Op]`.
    ///
    /// If memory bloat is a concern, consider using the [`Vm::exec_bytecode`]
    /// or [`Vm::exec_bytecode_iter`] methods which allow for providing a more
    /// compact representation of the operations in the form of mapped bytecode.
    pub async fn exec_ops<'a, S>(
        &mut self,
        ops: &[Op],
        access: Access<'a>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
    {
        self.exec(access, state_read, ops, op_gas_cost, gas_limit)
            .await
    }

    /// Execute the given mapped bytecode from the current state of the VM.
    ///
    /// Upon reaching a `Halt` operation or reaching the end of the operation
    /// sequence, returns the gas spent and the `Vm` will be left in the
    /// resulting state.
    ///
    /// This is a wrapper around [`Vm::exec`] that expects operation access in
    /// the form of [`&BytecodeMapped`][BytecodeMapped].
    ///
    /// This can be a more memory efficient alternative to [`Vm::exec_ops`] due
    /// to the compact representation of operations in the form of bytecode and
    /// indices.
    pub async fn exec_bytecode<'a, S, B>(
        &mut self,
        bytecode_mapped: &BytecodeMapped<B>,
        access: Access<'a>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
        B: core::ops::Deref<Target = [u8]>,
    {
        self.exec(access, state_read, bytecode_mapped, op_gas_cost, gas_limit)
            .await
    }

    /// Execute the given bytecode from the current state of the VM.
    ///
    /// Upon reaching a `Halt` operation or reaching the end of the operation
    /// sequence, returns the gas spent and the `Vm` will be left in the
    /// resulting state.
    ///
    /// The given bytecode will be mapped lazily during execution. This
    /// can be more efficient than pre-mapping the bytecode and using
    /// [`Vm::exec_bytecode`] in the case that execution may fail early.
    ///
    /// However, successful execution still requires building the full
    /// [`BytecodeMapped`] instance internally. So if bytecode has already been
    /// mapped, [`Vm::exec_bytecode`] should be preferred.
    pub async fn exec_bytecode_iter<'a, S, I>(
        &mut self,
        bytecode_iter: I,
        access: Access<'a>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
        I: IntoIterator<Item = u8>,
        I::IntoIter: Unpin,
    {
        let bytecode_lazy = BytecodeMappedLazy::new(bytecode_iter);
        self.exec(access, state_read, bytecode_lazy, op_gas_cost, gas_limit)
            .await
    }

    /// Execute over the given operation access from the current state of the VM.
    ///
    /// Upon reaching a `Halt` operation or reaching the end of the operation
    /// sequence, returns the gas spent and the `Vm` will be left in the
    /// resulting state.
    ///
    /// The type requirements for the `op_access` argument can make this
    /// finicky to use directly. You may prefer one of the convenience methods:
    ///
    /// - [`Vm::exec_ops`]
    /// - [`Vm::exec_bytecode`]
    /// - [`Vm::exec_bytecode_iter`]
    pub async fn exec<'a, S, OA>(
        &mut self,
        access: Access<'a>,
        state_read: &S,
        op_access: OA,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
        OA: OpAccess<Op = Op> + Unpin,
        OA::Error: Into<OpError<S::Error>>,
    {
        future::exec(self, access, state_read, op_access, op_gas_cost, gas_limit).await
    }
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
