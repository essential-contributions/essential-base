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

#[doc(inline)]
pub use bytecode::{BytecodeMapped, BytecodeMappedSlice};
use error::{MemoryError, OpError, OpSyncError, StateReadError};
#[doc(inline)]
pub use error::{MemoryResult, OpAsyncResult, OpResult, OpSyncResult, StateReadResult};
#[doc(inline)]
pub use essential_constraint_vm::{
    self as constraint, Access, SolutionAccess, Stack, StateSlotSlice, StateSlots,
};
#[doc(inline)]
pub use essential_state_asm as asm;
use essential_state_asm::Op;
pub use essential_types as types;
use essential_types::{ContentAddress, Word};
#[doc(inline)]
pub use future::ExecFuture;
pub use memory::Memory;
pub use state_read::StateRead;

mod bytecode;
mod ctrl_flow;
pub mod error;
mod future;
mod memory;
mod state_read;

/// The operation execution state of the State Read VM.
#[derive(Debug, Default, PartialEq)]
pub struct Vm {
    /// The program counter, i.e. index of the current operation within the program.
    pub pc: usize,
    /// The stack machine.
    pub stack: Stack,
    /// The program memory, primarily used for collecting the state being read.
    pub memory: Memory,
}

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

/// The set of operations performed synchronously.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub(crate) enum OpSync {
    /// All operations available to the constraint checker.
    Constraint(asm::Constraint),
    /// Operations for controlling the flow of the program.
    ControlFlow(asm::ControlFlow),
    /// Operations for controlling the flow of the program.
    Memory(asm::Memory),
}

/// The set of operations that are performed asynchronously.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub(crate) enum OpAsync {
    /// Read a range of words from state starting at the key.
    StateReadWordRange,
    /// Read a range of words from external state starting at the key.
    StateReadWordRangeExt,
}

/// Types that provide access to operations.
///
/// Implementations are included for `&[Op]`, `BytecodeMapped` and more.
pub trait OpAccess {
    /// Any error that might occur during access.
    type Error: std::error::Error;
    /// Access the operation at the given index.
    ///
    /// Mutable access to self is required in case operations are lazily parsed.
    ///
    /// Any implementation should ensure the same index always returns the same operation.
    fn op_access(&mut self, index: usize) -> Option<Result<Op, Self::Error>>;
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
    pub async fn exec_bytecode<'a, S>(
        &mut self,
        bytecode_mapped: &BytecodeMapped,
        access: Access<'a>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
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
        /// A type wrapper around `BytecodeMapped` that lazily constructs the
        /// map from the given bytecode as operations are accessed.
        struct BytecodeMappedLazy<I> {
            mapped: BytecodeMapped,
            iter: I,
        }

        // Op access lazily populates operations from the given byte iterator
        // as necessary.
        impl<I> OpAccess for BytecodeMappedLazy<I>
        where
            I: Iterator<Item = u8>,
        {
            type Error = asm::FromBytesError;
            fn op_access(&mut self, index: usize) -> Option<Result<Op, Self::Error>> {
                loop {
                    match self.mapped.op(index) {
                        Some(op) => return Some(Ok(op)),
                        None => match Op::from_bytes(&mut self.iter)? {
                            Err(err) => return Some(Err(err)),
                            Ok(op) => self.mapped.push_op(op),
                        },
                    }
                }
            }
        }

        let bytecode_lazy = BytecodeMappedLazy {
            mapped: BytecodeMapped::default(),
            iter: bytecode_iter.into_iter(),
        };
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
        OA: OpAccess + Unpin,
        OA::Error: Into<OpError<S::Error>>,
    {
        future::exec(self, access, state_read, op_access, op_gas_cost, gas_limit).await
    }

    /// Consumes the `Vm` and returns the read state slots.
    ///
    /// The returned slots correspond directly with the current memory content.
    pub fn into_state_slots(self) -> Vec<Option<Word>> {
        self.memory.into()
    }
}

impl From<Op> for OpKind {
    fn from(op: Op) -> Self {
        match op {
            Op::Constraint(op) => OpKind::Sync(OpSync::Constraint(op)),
            Op::ControlFlow(op) => OpKind::Sync(OpSync::ControlFlow(op)),
            Op::Memory(op) => OpKind::Sync(OpSync::Memory(op)),
            Op::WordRange => OpKind::Async(OpAsync::StateReadWordRange),
            Op::WordRangeExtern => OpKind::Async(OpAsync::StateReadWordRangeExt),
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

impl<'a> OpAccess for &'a [Op] {
    type Error = core::convert::Infallible;
    fn op_access(&mut self, index: usize) -> Option<Result<Op, Self::Error>> {
        self.get(index).copied().map(Ok)
    }
}

impl<'a> OpAccess for &'a BytecodeMapped {
    type Error = core::convert::Infallible;
    fn op_access(&mut self, index: usize) -> Option<Result<Op, Self::Error>> {
        self.op(index).map(Ok)
    }
}

/// Step forward the VM by a single synchronous operation.
///
/// Returns a `Some(usize)` representing the new program counter resulting from
/// this step, or `None` in the case that execution has halted.
pub(crate) fn step_op_sync(op: OpSync, access: Access, vm: &mut Vm) -> OpSyncResult<Option<usize>> {
    match op {
        OpSync::Constraint(op) => constraint::step_op(access, op, &mut vm.stack)?,
        OpSync::ControlFlow(op) => return step_op_ctrl_flow(op, vm).map_err(From::from),
        OpSync::Memory(op) => step_op_memory(op, &mut *vm)?,
    }
    // Every operation besides control flow steps forward program counter by 1.
    let new_pc = vm.pc.checked_add(1).ok_or(OpSyncError::PcOverflow)?;
    Ok(Some(new_pc))
}

/// Step forward state reading by the given control flow operation.
///
/// Returns a `bool` indicating whether or not to continue execution.
pub(crate) fn step_op_ctrl_flow(op: asm::ControlFlow, vm: &mut Vm) -> OpSyncResult<Option<usize>> {
    match op {
        asm::ControlFlow::Jump => ctrl_flow::jump(vm).map(Some).map_err(From::from),
        asm::ControlFlow::JumpIf => ctrl_flow::jump_if(vm).map(Some),
        asm::ControlFlow::Halt => Ok(None),
    }
}

/// Step forward state reading by the given memory operation.
pub(crate) fn step_op_memory(op: asm::Memory, vm: &mut Vm) -> OpSyncResult<()> {
    match op {
        asm::Memory::Alloc => memory::alloc(vm),
        asm::Memory::Capacity => memory::capacity(vm),
        asm::Memory::Clear => memory::clear(vm),
        asm::Memory::ClearRange => memory::clear_range(vm),
        asm::Memory::Free => memory::free(vm),
        asm::Memory::IsSome => memory::is_some(vm),
        asm::Memory::Length => memory::length(vm),
        asm::Memory::Load => memory::load(vm),
        asm::Memory::Push => memory::push(vm),
        asm::Memory::PushNone => memory::push_none(vm),
        asm::Memory::Store => memory::store(vm),
        asm::Memory::Truncate => memory::truncate(vm),
    }
}
