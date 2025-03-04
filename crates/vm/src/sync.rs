//! Items related to stepping forward VM execution by synchronous operations.

use crate::{
    access, alu, asm, crypto,
    error::{
        EvalSyncError, EvalSyncResult, ExecSyncError, ExecSyncResult, OpSyncError, OpSyncResult,
    },
    pred, repeat, total_control_flow,
    types::convert::bool_from_word,
    Access, LazyCache, Memory, OpAccess, OpSync, ProgramControlFlow, Repeat, Stack, Vm,
};

impl From<asm::Access> for OpSync {
    fn from(op: asm::Access) -> Self {
        Self::Access(op)
    }
}

impl From<asm::Alu> for OpSync {
    fn from(op: asm::Alu) -> Self {
        Self::Alu(op)
    }
}

impl From<asm::TotalControlFlow> for OpSync {
    fn from(op: asm::TotalControlFlow) -> Self {
        Self::ControlFlow(op)
    }
}

impl From<asm::Crypto> for OpSync {
    fn from(op: asm::Crypto) -> Self {
        Self::Crypto(op)
    }
}

impl From<asm::Memory> for OpSync {
    fn from(op: asm::Memory) -> Self {
        Self::Memory(op)
    }
}

impl From<asm::Pred> for OpSync {
    fn from(op: asm::Pred) -> Self {
        Self::Pred(op)
    }
}

impl From<asm::Stack> for OpSync {
    fn from(op: asm::Stack) -> Self {
        Self::Stack(op)
    }
}

/// Evaluate a slice of synchronous operations and return their boolean result.
///
/// This is the same as [`exec_ops`], but retrieves the boolean result from the resulting stack.
pub fn eval_ops(ops: &[OpSync], access: Access) -> EvalSyncResult<bool> {
    eval(ops, access)
}

/// Evaluate the operations of a single synchronous program and return its boolean result.
///
/// This is the same as [`exec`], but retrieves the boolean result from the resulting stack.
pub fn eval<OA>(op_access: OA, access: Access) -> EvalSyncResult<bool>
where
    OA: OpAccess<Op = OpSync>,
    OA::Error: Into<OpSyncError>,
{
    let stack = exec(op_access, access)?;
    let word = match stack.last() {
        Some(&w) => w,
        None => return Err(EvalSyncError::InvalidEvaluation(stack)),
    };
    bool_from_word(word).ok_or_else(|| EvalSyncError::InvalidEvaluation(stack))
}

/// Execute a slice of synchronous operations and return the resulting stack.
pub fn exec_ops(ops: &[OpSync], access: Access) -> ExecSyncResult<Stack> {
    exec(ops, access)
}

/// Execute the given synchronous operations and return the resulting stack.
pub fn exec<OA>(mut op_access: OA, access: Access) -> ExecSyncResult<Stack>
where
    OA: OpAccess<Op = OpSync>,
    OA::Error: Into<OpSyncError>,
{
    let mut pc = 0;
    let mut stack = Stack::default();
    let mut memory = Memory::new();
    let mut repeat = Repeat::new();
    let cache = LazyCache::new();
    while let Some(res) = op_access.op_access(pc) {
        let op = res.map_err(|err| ExecSyncError(pc, err.into()))?;

        let res = step_op(access, op, &mut stack, &mut memory, pc, &mut repeat, &cache);

        #[cfg(feature = "tracing")]
        crate::trace_op_res(&mut op_access, pc, &stack, &memory, res.as_ref());

        let update = match res {
            Ok(update) => update,
            Err(err) => return Err(ExecSyncError(pc, err)),
        };

        match update {
            Some(ProgramControlFlow::Pc(new_pc)) => pc = new_pc,
            Some(ProgramControlFlow::Halt) => break,
            None => pc += 1,
        }
    }
    Ok(stack)
}

/// Step forward the VM by a single synchronous operation.
///
/// Returns a `Some(usize)` representing the new program counter resulting from
/// this step, or `None` in the case that execution has halted.
pub fn step_op_sync(op: OpSync, access: Access, vm: &mut Vm) -> OpSyncResult<Option<usize>> {
    let Vm {
        stack,
        repeat,
        pc,
        memory,
        cache,
        ..
    } = vm;
    match step_op(access, op, stack, memory, *pc, repeat, cache)? {
        Some(ProgramControlFlow::Pc(pc)) => return Ok(Some(pc)),
        Some(ProgramControlFlow::Halt) => return Ok(None),
        None => (),
    }
    // Every operation besides control flow steps forward program counter by 1.
    let new_pc = vm.pc.checked_add(1).ok_or(OpSyncError::PcOverflow)?;
    Ok(Some(new_pc))
}

/// Step forward execution by the given synchronous operation.
pub fn step_op(
    access: Access,
    op: OpSync,
    stack: &mut Stack,
    memory: &mut Memory,
    pc: usize,
    repeat: &mut Repeat,
    cache: &LazyCache,
) -> OpSyncResult<Option<ProgramControlFlow>> {
    match op {
        OpSync::Access(op) => step_op_access(access, op, stack, repeat, cache).map(|_| None),
        OpSync::Alu(op) => step_op_alu(op, stack).map(|_| None),
        OpSync::Crypto(op) => step_op_crypto(op, stack).map(|_| None),
        OpSync::Pred(op) => step_op_pred(op, stack).map(|_| None),
        OpSync::Stack(op) => step_op_stack(op, pc, stack, repeat),
        OpSync::ControlFlow(op) => step_op_total_control_flow(op, stack, pc),
        OpSync::Memory(op) => step_op_memory(op, stack, memory).map(|_| None),
    }
}

/// Step forward execution by the given access operation.
pub fn step_op_access(
    access: Access,
    op: asm::Access,
    stack: &mut Stack,
    repeat: &mut Repeat,
    cache: &LazyCache,
) -> OpSyncResult<()> {
    match op {
        asm::Access::PredicateData => {
            access::predicate_data(&access.this_solution().predicate_data, stack)
        }
        asm::Access::PredicateDataLen => {
            access::predicate_data_len(&access.this_solution().predicate_data, stack)
                .map_err(From::from)
        }
        asm::Access::PredicateDataSlots => {
            access::predicate_data_slots(stack, &access.this_solution().predicate_data)
        }
        asm::Access::MutKeys => access::push_mut_keys(access, stack),
        asm::Access::ThisAddress => access::this_address(access.this_solution(), stack),
        asm::Access::ThisContractAddress => {
            access::this_contract_address(access.this_solution(), stack)
        }
        asm::Access::RepeatCounter => access::repeat_counter(stack, repeat),
        asm::Access::PredicateExists => access::predicate_exists(stack, access.solutions, cache),
    }
}

/// Step forward execution by the given ALU operation.
pub fn step_op_alu(op: asm::Alu, stack: &mut Stack) -> OpSyncResult<()> {
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

/// Step forward execution by the given crypto operation.
pub fn step_op_crypto(op: asm::Crypto, stack: &mut Stack) -> OpSyncResult<()> {
    match op {
        asm::Crypto::Sha256 => crypto::sha256(stack),
        asm::Crypto::VerifyEd25519 => crypto::verify_ed25519(stack),
        asm::Crypto::RecoverSecp256k1 => crypto::recover_secp256k1(stack),
    }
}

/// Step forward execution by the given predicate operation.
pub fn step_op_pred(op: asm::Pred, stack: &mut Stack) -> OpSyncResult<()> {
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

/// Step forward execution by the given stack operation.
pub fn step_op_stack(
    op: asm::Stack,
    pc: usize,
    stack: &mut Stack,
    repeat: &mut Repeat,
) -> OpSyncResult<Option<ProgramControlFlow>> {
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

/// Step forward execution by the given total control flow operation.
pub fn step_op_total_control_flow(
    op: asm::TotalControlFlow,
    stack: &mut Stack,
    pc: usize,
) -> OpSyncResult<Option<ProgramControlFlow>> {
    match op {
        asm::TotalControlFlow::JumpIf => total_control_flow::jump_if(stack, pc),
        asm::TotalControlFlow::HaltIf => total_control_flow::halt_if(stack),
        asm::TotalControlFlow::Halt => Ok(Some(ProgramControlFlow::Halt)),
        asm::TotalControlFlow::PanicIf => total_control_flow::panic_if(stack).map(|_| None),
    }
}

/// Step forward execution by the given memory operation.
pub fn step_op_memory(op: asm::Memory, stack: &mut Stack, memory: &mut Memory) -> OpSyncResult<()> {
    match op {
        asm::Memory::Alloc => {
            let w = stack.pop()?;
            let len = memory.len()?;
            memory.alloc(w)?;
            Ok(stack.push(len)?)
        }
        asm::Memory::Store => {
            let [w, addr] = stack.pop2()?;
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
                Ok::<_, OpSyncError>(())
            })?;
            Ok(())
        }
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use crate::{
        types::{solution::Solution, ContentAddress, PredicateAddress},
        *,
    };
    use asm::Word;
    use std::collections::HashSet;

    pub(crate) const TEST_SET_CA: ContentAddress = ContentAddress([0xFF; 32]);
    pub(crate) const TEST_PREDICATE_CA: ContentAddress = ContentAddress([0xAA; 32]);
    pub(crate) const TEST_PREDICATE_ADDR: PredicateAddress = PredicateAddress {
        contract: TEST_SET_CA,
        predicate: TEST_PREDICATE_CA,
    };
    pub(crate) const TEST_SOLUTION: Solution = Solution {
        predicate_to_solve: TEST_PREDICATE_ADDR,
        predicate_data: vec![],
        state_mutations: vec![],
    };

    pub(crate) fn test_empty_keys() -> &'static HashSet<&'static [Word]> {
        static INSTANCE: std::sync::LazyLock<HashSet<&[Word]>> =
            std::sync::LazyLock::new(|| HashSet::with_capacity(0));
        &INSTANCE
    }

    pub(crate) fn test_solutions() -> &'static [Solution] {
        static INSTANCE: std::sync::LazyLock<[Solution; 1]> =
            std::sync::LazyLock::new(|| [TEST_SOLUTION]);
        &*INSTANCE
    }

    pub(crate) fn test_access() -> &'static Access<'static> {
        static INSTANCE: std::sync::LazyLock<Access> = std::sync::LazyLock::new(|| Access {
            solutions: test_solutions(),
            index: 0,
            mutable_keys: test_empty_keys(),
        });
        &INSTANCE
    }
}
