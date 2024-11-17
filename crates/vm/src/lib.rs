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
pub use error::{OpAsyncResult, OpResult, OpSyncResult, StateReadResult};
use error::{OpError, OpSyncError, StateReadError};
#[doc(inline)]
pub use essential_asm as asm;
use essential_state_asm::Op;
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

/// Step forward constraint checking by the given operation.
pub fn step_op(
    access: Access,
    op: asm::Constraint,
    stack: &mut Stack,
    memory: &mut Memory,
    pc: usize,
    repeat: &mut Repeat,
    cache: &LazyCache,
) -> OpResult<Option<ProgramControlFlow>> {
    match op {
        Op::Access(op) => step_op_access(access, op, stack, repeat, cache).map(|_| None),
        Op::Alu(op) => step_op_alu(op, stack).map(|_| None),
        Op::Crypto(op) => step_op_crypto(op, stack).map(|_| None),
        Op::Pred(op) => step_op_pred(op, stack).map(|_| None),
        Op::Stack(op) => step_op_stack(op, pc, stack, repeat),
        Op::TotalControlFlow(op) => step_on_total_control_flow(op, stack, pc),
        Op::Memory(op) => step_on_temporary(op, stack, memory).map(|_| None),
    }
}

/// Step forward constraint checking by the given access operation.
pub fn step_op_access(
    access: Access,
    op: asm::Access,
    stack: &mut Stack,
    repeat: &mut Repeat,
    cache: &LazyCache,
) -> OpResult<()> {
    match op {
        asm::Access::DecisionVar => {
            access::decision_var(&access.this_data().decision_variables, stack)
        }
        asm::Access::DecisionVarLen => {
            access::decision_var_len(&access.this_data().decision_variables, stack)
        }
        asm::Access::DecisionVarSlots => {
            access::decision_var_slots(stack, &access.this_data().decision_variables)
        }
        asm::Access::MutKeys => access::push_mut_keys(access, stack),
        asm::Access::ThisAddress => access::this_address(access.this_data(), stack),
        asm::Access::ThisContractAddress => {
            access::this_contract_address(access.this_data(), stack)
        }
        asm::Access::RepeatCounter => access::repeat_counter(stack, repeat),
        asm::Access::PredicateExists => access::predicate_exists(stack, access.data, cache),
    }
}

/// Step forward constraint checking by the given ALU operation.
pub fn step_op_alu(op: asm::Alu, stack: &mut Stack) -> OpResult<()> {
    match op {
        asm::Alu::Add => stack.pop2_push1(alu::add),
        asm::Alu::Sub => stack.pop2_push1(alu::sub),
        asm::Alu::Mul => stack.pop2_push1(alu::mul),
        asm::Alu::Div => stack.pop2_push1(alu::div),
        asm::Alu::Mod => stack.pop2_push1(alu::mod_),
        asm::Alu::Shl => stack.pop2_push1(alu::shl),
        asm::Alu::Shr => stack.pop2_push1(alu::shr),
        asm::Alu::ShrI => stack.pop2_push1(alu::arithmetic_shr),
    }
}

/// Step forward constraint checking by the given crypto operation.
pub fn step_op_crypto(op: asm::Crypto, stack: &mut Stack) -> OpResult<()> {
    match op {
        asm::Crypto::Sha256 => crypto::sha256(stack),
        asm::Crypto::VerifyEd25519 => crypto::verify_ed25519(stack),
        asm::Crypto::RecoverSecp256k1 => crypto::recover_secp256k1(stack),
    }
}

/// Step forward constraint checking by the given predicate operation.
pub fn step_op_pred(op: asm::Pred, stack: &mut Stack) -> OpResult<()> {
    match op {
        asm::Pred::Eq => stack.pop2_push1(|a, b| Ok((a == b).into())),
        asm::Pred::EqRange => pred::eq_range(stack),
        asm::Pred::Gt => stack.pop2_push1(|a, b| Ok((a > b).into())),
        asm::Pred::Lt => stack.pop2_push1(|a, b| Ok((a < b).into())),
        asm::Pred::Gte => stack.pop2_push1(|a, b| Ok((a >= b).into())),
        asm::Pred::Lte => stack.pop2_push1(|a, b| Ok((a <= b).into())),
        asm::Pred::And => stack.pop2_push1(|a, b| Ok((a != 0 && b != 0).into())),
        asm::Pred::Or => stack.pop2_push1(|a, b| Ok((a != 0 || b != 0).into())),
        asm::Pred::Not => stack.pop1_push1(|a| Ok((a == 0).into())),
        asm::Pred::EqSet => pred::eq_set(stack),
        asm::Pred::BitAnd => stack.pop2_push1(|a, b| Ok(a & b)),
        asm::Pred::BitOr => stack.pop2_push1(|a, b| Ok(a | b)),
    }
}

/// Step forward constraint checking by the given stack operation.
pub fn step_op_stack(
    op: asm::Stack,
    pc: usize,
    stack: &mut Stack,
    repeat: &mut Repeat,
) -> OpResult<Option<ProgramControlFlow>> {
    if let asm::Stack::RepeatEnd = op {
        return Ok(repeat.repeat()?.map(ProgramControlFlow::Pc));
    }
    let r = match op {
        asm::Stack::Dup => stack.pop1_push2(|w| Ok([w, w])),
        asm::Stack::DupFrom => stack.dup_from().map_err(From::from),
        asm::Stack::Push(word) => stack.push(word).map_err(From::from),
        asm::Stack::Pop => stack.pop().map(|_| ()).map_err(From::from),
        asm::Stack::Swap => stack.pop2_push2(|a, b| Ok([b, a])),
        asm::Stack::SwapIndex => stack.swap_index().map_err(From::from),
        asm::Stack::Select => stack.select().map_err(From::from),
        asm::Stack::SelectRange => stack.select_range().map_err(From::from),
        asm::Stack::Repeat => repeat::repeat(pc, stack, repeat),
        asm::Stack::Reserve => stack.reserve_zeroed().map_err(From::from),
        asm::Stack::Load => stack.load().map_err(From::from),
        asm::Stack::Store => stack.store().map_err(From::from),
        asm::Stack::RepeatEnd => unreachable!(),
    };
    r.map(|_| None)
}

/// Step forward constraint checking by the given total control flow operation.
pub fn step_on_total_control_flow(
    op: asm::TotalControlFlow,
    stack: &mut Stack,
    pc: usize,
) -> OpResult<Option<ProgramControlFlow>> {
    match op {
        asm::TotalControlFlow::JumpForwardIf => total_control_flow::jump_forward_if(stack, pc),
        asm::TotalControlFlow::HaltIf => total_control_flow::halt_if(stack),
        asm::TotalControlFlow::Halt => Ok(Some(ProgramControlFlow::Halt)),
        asm::TotalControlFlow::PanicIf => total_control_flow::panic_if(stack).map(|_| None),
    }
}

/// Step forward constraint checking by the given temporary operation.
pub fn step_on_temporary(op: asm::Memory, stack: &mut Stack, memory: &mut Memory) -> OpResult<()> {
    match op {
        asm::Memory::Alloc => {
            let w = stack.pop()?;
            let len = memory.len()?;
            memory.alloc(w)?;
            Ok(stack.push(len)?)
        }
        asm::Memory::Store => {
            let [addr, w] = stack.pop2()?;
            memory.store(addr, w)?;
            Ok(())
        }
        asm::Memory::Load => stack.pop1_push1(|addr| {
            let w = memory.load(addr)?;
            Ok(w)
        }),
        asm::Memory::Free => {
            let addr = stack.pop()?;
            memory.free(addr)?;
            Ok(())
        }
        asm::Memory::LoadRange => {
            let [addr, size] = stack.pop2()?;
            let words = memory.load_range(addr, size)?;
            Ok(stack.extend(words)?)
        }
        asm::Memory::StoreRange => {
            let addr = stack.pop()?;
            stack.pop_len_words(|words| {
                memory.store_range(addr, words)?;
                Ok::<_, OpError>(())
            })?;
            Ok(())
        }
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use std::collections::HashSet;

    use asm::Word;

    use crate::{
        types::{solution::SolutionData, ContentAddress, PredicateAddress},
        *,
    };

    pub(crate) const TEST_SET_CA: ContentAddress = ContentAddress([0xFF; 32]);
    pub(crate) const TEST_PREDICATE_CA: ContentAddress = ContentAddress([0xAA; 32]);
    pub(crate) const TEST_PREDICATE_ADDR: PredicateAddress = PredicateAddress {
        contract: TEST_SET_CA,
        predicate: TEST_PREDICATE_CA,
    };
    pub(crate) const TEST_SOLUTION_DATA: SolutionData = SolutionData {
        predicate_to_solve: TEST_PREDICATE_ADDR,
        decision_variables: vec![],
        state_mutations: vec![],
    };

    pub(crate) fn test_empty_keys() -> &'static HashSet<&'static [Word]> {
        static INSTANCE: std::sync::LazyLock<HashSet<&[Word]>> =
            std::sync::LazyLock::new(|| HashSet::with_capacity(0));
        &INSTANCE
    }

    pub(crate) fn test_solution_data_arr() -> &'static [SolutionData] {
        static INSTANCE: std::sync::LazyLock<[SolutionData; 1]> =
            std::sync::LazyLock::new(|| [TEST_SOLUTION_DATA]);
        &*INSTANCE
    }

    pub(crate) fn test_access() -> &'static Access<'static> {
        static INSTANCE: std::sync::LazyLock<Access> = std::sync::LazyLock::new(|| Access {
            data: test_solution_data_arr(),
            index: 0,
            mutable_keys: test_empty_keys(),
        });
        &INSTANCE
    }
}

#[cfg(test)]
mod pred_tests {
    use crate::{
        asm::{Pred, Stack},
        test_util::*,
        *,
    };

    #[test]
    fn pred_eq_false() {
        let ops = &[
            Stack::Push(6).into(),
            Stack::Push(7).into(),
            Pred::Eq.into(),
        ];
        assert!(!eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_eq_true() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(42).into(),
            Pred::Eq.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_gt_false() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(7).into(),
            Pred::Gt.into(),
        ];
        assert!(!eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_gt_true() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(6).into(),
            Pred::Gt.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_lt_false() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(7).into(),
            Pred::Lt.into(),
        ];
        assert!(!eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_lt_true() {
        let ops = &[
            Stack::Push(6).into(),
            Stack::Push(7).into(),
            Pred::Lt.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_gte_false() {
        let ops = &[
            Stack::Push(6).into(),
            Stack::Push(7).into(),
            Pred::Gte.into(),
        ];
        assert!(!eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_gte_true() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(7).into(),
            Pred::Gte.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
        let ops = &[
            Stack::Push(8).into(),
            Stack::Push(7).into(),
            Pred::Gte.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_lte_false() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(6).into(),
            Pred::Lte.into(),
        ];
        assert!(!eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_lte_true() {
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(7).into(),
            Pred::Lte.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
        let ops = &[
            Stack::Push(7).into(),
            Stack::Push(8).into(),
            Pred::Lte.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_and_true() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(42).into(),
            Pred::And.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_and_false() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(0).into(),
            Pred::And.into(),
        ];
        assert!(!eval_ops(ops, *test_access()).unwrap());
        let ops = &[
            Stack::Push(0).into(),
            Stack::Push(0).into(),
            Pred::And.into(),
        ];
        assert!(!eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_or_true() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(42).into(),
            Pred::Or.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
        let ops = &[
            Stack::Push(0).into(),
            Stack::Push(42).into(),
            Pred::Or.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(0).into(),
            Pred::Or.into(),
        ];
        assert!(eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_or_false() {
        let ops = &[
            Stack::Push(0).into(),
            Stack::Push(0).into(),
            Pred::Or.into(),
        ];
        assert!(!eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_not_true() {
        let ops = &[Stack::Push(0).into(), Pred::Not.into()];
        assert!(eval_ops(ops, *test_access()).unwrap());
    }

    #[test]
    fn pred_not_false() {
        let ops = &[Stack::Push(42).into(), Pred::Not.into()];
        assert!(!eval_ops(ops, *test_access()).unwrap());
    }
}
