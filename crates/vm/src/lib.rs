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
//! There are two primary methods available for executing operations:
//!
//! - [`Vm::exec_ops`]
//! - [`Vm::exec_bytecode`]
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

pub use access::Access;
pub use cached::LazyCache;
#[doc(inline)]
pub use essential_asm::{self as asm, Op};
pub use essential_types as types;
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
pub use state_read::StateReads;
#[doc(inline)]
pub use total_control_flow::ProgramControlFlow;
#[doc(inline)]
pub use vm::Vm;

mod access;
mod alu;
pub mod bytecode;
mod cached;
mod compute;
mod crypto;
pub mod error;
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

#[cfg(test)]
pub(crate) mod utils {
    use crate::{StateRead, StateReads};

    pub struct EmptyState;
    impl StateRead for EmptyState {
        type Error = String;

        fn key_range(
            &self,
            _contract_addr: essential_types::ContentAddress,
            _key: essential_types::Key,
            _num_values: usize,
        ) -> Result<Vec<Vec<essential_asm::Word>>, Self::Error> {
            Ok(vec![])
        }
    }

    impl StateReads for EmptyState {
        type Error = String;
        type Pre = Self;
        type Post = Self;

        fn pre(&self) -> &Self::Pre {
            self
        }

        fn post(&self) -> &Self::Post {
            self
        }
    }
}

/// Shorthand for the `BytecodeMapped` type representing a mapping to/from [`Op`]s.
pub type BytecodeMapped<Bytes = Vec<u8>> = bytecode::BytecodeMapped<Op, Bytes>;
/// Shorthand for the `BytecodeMappedSlice` type for mapping [`Op`]s.
pub type BytecodeMappedSlice<'a> = bytecode::BytecodeMappedSlice<'a, Op>;

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
    oa: &OA,
    pc: usize,
    stack: &Stack,
    memory: &Memory,
    op_res: &Result<T, E>,
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
