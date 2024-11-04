//! The essential constraint checking implementation.
//!
//! ## Checking Predicates
//!
//! The primary entrypoint for this crate is the [`check_predicate`] function
//! which allows for checking a contract of constraints associated with a single
//! predicate against some provided solution data and state slot mutations in
//! parallel.
//!
//! ## Checking Individual Constraints
//!
//! Functions are also exposed for checking constraints individually.
//!
//! - The [`exec_bytecode`], [`exec_bytecode_iter`] and [`exec_ops`] functions
//!   allow for executing the constraint and returning the resulting `Stack`.
//! - The [`eval_bytecode`], [`eval_bytecode_iter`] and [`eval_ops`] functions
//!   are similar to their `exec_*` counterparts, but expect the top of
//!   the `Stack` to contain a single boolean value indicating whether the
//!   constraint was satisfied (`0` for `false`, `1` for `true`) and returns
//!   this value.
//!
//! ## Performing a Single Operation
//!
//! The [`step_op`] function (and related `step_op_*` functions) are exposed to
//! allow for applying a single operation to the given stack. This can be useful
//! in the case of integrating constraint operations in a downstream VM (e.g.
//! the essential state read VM).
//!
//! ## Understanding the Assembly
//!
//! The `essential-constraint-asm` crate is re-exported as the [`asm`] module.
//! See [this module's documentation][asm] for information about the expected
//! behaviour of individual operations.
#![deny(missing_docs, unsafe_code)]

pub use access::{mut_keys, mut_keys_set, mut_keys_slices, Access};
#[doc(inline)]
pub use bytecode::{BytecodeMapped, BytecodeMappedLazy, BytecodeMappedSlice};
pub use cached::LazyCache;
#[doc(inline)]
pub use error::{CheckResult, ConstraintResult, OpResult, StackResult};
use error::{ConstraintError, ConstraintErrors, ConstraintsUnsatisfied, OpError};
#[doc(inline)]
pub use essential_constraint_asm as asm;
use essential_constraint_asm::Op;
pub use essential_types as types;
use essential_types::{convert::bool_from_word, ConstraintBytecode};
#[doc(inline)]
pub use memory::Memory;
#[doc(inline)]
pub use op_access::OpAccess;
#[doc(inline)]
pub use repeat::Repeat;
#[doc(inline)]
pub use stack::Stack;
#[doc(inline)]
pub use total_control_flow::ProgramControlFlow;

mod access;
mod alu;
mod bytecode;
mod cached;
mod crypto;
pub mod error;
mod memory;
mod op_access;
mod pred;
mod repeat;
mod sets;
mod stack;
mod total_control_flow;

/// Check whether the constraints of a single predicate are met for the given
/// solution data and state slot mutations. All constraints are checked in
/// parallel.
///
/// In the case that one or more constraints fail or are unsatisfied, the
/// whole contract of failed/unsatisfied constraint indices are returned within the
/// [`CheckError`][error::CheckError] type.
///
/// The predicate is considered to be satisfied if this function returns `Ok(())`.
pub fn check_predicate(predicate: &[ConstraintBytecode], access: Access) -> CheckResult<()> {
    use rayon::{iter::Either, prelude::*};
    let (failed, unsatisfied): (Vec<_>, Vec<_>) = predicate
        .par_iter()
        .map(|bytecode| eval_bytecode_iter(bytecode.iter().copied(), access))
        .enumerate()
        .filter_map(|(i, constraint_res)| match constraint_res {
            Err(err) => Some(Either::Left((i, err))),
            Ok(b) if !b => Some(Either::Right(i)),
            _ => None,
        })
        .partition_map(|either| either);
    if !failed.is_empty() {
        return Err(ConstraintErrors(failed).into());
    }
    if !unsatisfied.is_empty() {
        return Err(ConstraintsUnsatisfied(unsatisfied).into());
    }
    Ok(())
}

/// Evaluate the bytecode of a single constraint and return its boolean result.
///
/// This is the same as [`exec_bytecode`], but retrieves the boolean result from the resulting stack.
pub fn eval_bytecode(bytes: &BytecodeMapped<Op>, access: Access) -> ConstraintResult<bool> {
    eval(bytes, access)
}

/// Evaluate the bytecode of a single constraint and return its boolean result.
///
/// This is the same as [`eval_bytecode`], but lazily constructs the bytecode
/// mapping as bytes are parsed.
pub fn eval_bytecode_iter<I>(bytes: I, access: Access) -> ConstraintResult<bool>
where
    I: IntoIterator<Item = u8>,
{
    eval(BytecodeMappedLazy::new(bytes), access)
}

/// Evaluate the operations of a single constraint and return its boolean result.
///
/// This is the same as [`exec_ops`], but retrieves the boolean result from the resulting stack.
pub fn eval_ops(ops: &[Op], access: Access) -> ConstraintResult<bool> {
    eval(ops, access)
}

/// Evaluate the operations of a single constraint and return its boolean result.
///
/// This is the same as [`exec`], but retrieves the boolean result from the resulting stack.
pub fn eval<OA>(op_access: OA, access: Access) -> ConstraintResult<bool>
where
    OA: OpAccess<Op = Op>,
    OA::Error: Into<error::OpError>,
{
    let stack = exec(op_access, access)?;
    let word = match stack.last() {
        Some(&w) => w,
        None => return Err(ConstraintError::InvalidEvaluation(stack)),
    };
    bool_from_word(word).ok_or_else(|| ConstraintError::InvalidEvaluation(stack))
}

/// Execute the bytecode of a constraint and return the resulting stack.
pub fn exec_bytecode(bytes: &BytecodeMapped<Op>, access: Access) -> ConstraintResult<Stack> {
    exec(bytes, access)
}

/// Execute the bytecode of a constraint and return the resulting stack.
///
/// This is the same as [`exec_bytecode`], but lazily constructs the bytecode
/// mapping as bytes are parsed.
pub fn exec_bytecode_iter<I>(bytes: I, access: Access) -> ConstraintResult<Stack>
where
    I: IntoIterator<Item = u8>,
{
    exec(BytecodeMappedLazy::new(bytes), access)
}

/// Execute the operations of a constraint and return the resulting stack.
pub fn exec_ops(ops: &[Op], access: Access) -> ConstraintResult<Stack> {
    exec(ops, access)
}

/// Execute the operations of a constraint and return the resulting stack.
pub fn exec<OA>(mut op_access: OA, access: Access) -> ConstraintResult<Stack>
where
    OA: OpAccess<Op = Op>,
    OA::Error: Into<error::OpError>,
{
    let mut pc = 0;
    let mut stack = Stack::default();
    let mut memory = Memory::new();
    let mut repeat = Repeat::new();
    let cache = LazyCache::new();
    while let Some(res) = op_access.op_access(pc) {
        let op = res.map_err(|err| ConstraintError::Op(pc, err.into()))?;

        let res = step_op(access, op, &mut stack, &mut memory, pc, &mut repeat, &cache);

        #[cfg(feature = "tracing")]
        trace_op_res(pc, &op, &stack, &memory, res.as_ref());

        let update = match res {
            Ok(update) => update,
            Err(err) => return Err(ConstraintError::Op(pc, err)),
        };

        match update {
            Some(ProgramControlFlow::Pc(new_pc)) => pc = new_pc,
            Some(ProgramControlFlow::Halt) => break,
            None => pc += 1,
        }
    }
    Ok(stack)
}

/// Trace the operation at the given program counter.
///
/// In the success case, also emits the resulting stack.
///
/// In the error case, emits a debug log with the error.
#[cfg(feature = "tracing")]
fn trace_op_res<T, E>(pc: usize, op: &Op, stack: &Stack, memory: &Memory, op_res: Result<T, E>)
where
    E: core::fmt::Display,
{
    let pc_op = format!("0x{pc:02X}: {op:?}");
    match op_res {
        Ok(_) => {
            tracing::trace!("{pc_op}\n  ├── {:?}\n  └── {:?}", &stack, &memory)
        }
        Err(ref err) => {
            tracing::trace!("{pc_op}");
            tracing::debug!("{err}");
        }
    }
}

/// Step forward constraint checking by the given operation.
pub fn step_op(
    access: Access,
    op: Op,
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
