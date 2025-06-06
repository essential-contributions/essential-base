//! Items related to stepping forward VM execution by synchronous operations.

use crate::{
    access, alu, asm,
    compute::ComputeInputs,
    crypto,
    error::{OpError, OpResult, ParentMemoryError},
    pred, repeat, total_control_flow, Access, GasLimit, LazyCache, Memory, OpAccess, OpGasCost,
    ProgramControlFlow, Repeat, Stack, StateReads, Vm,
};
use essential_asm::Op;
use essential_types::ContentAddress;
use std::sync::Arc;

/// Step forward execution by the given synchronous operation.
/// This includes the synchronous state read operation.
pub fn step_op<S, OA>(
    access: Access,
    op: Op,
    vm: &mut Vm,
    state: &S,
    op_access: OA,
    op_gas_cost: &impl OpGasCost,
    gas_limit: GasLimit,
) -> OpResult<Option<ProgramControlFlow>, S::Error>
where
    S: StateReads,
    OA: OpAccess<Op = Op>,
    OA::Error: Into<OpError<S::Error>>,
{
    let r = match op {
        Op::Access(op) => step_op_access(access, op, &mut vm.stack, &mut vm.repeat, &vm.cache)
            .map(|_| None)
            .map_err(OpError::from_infallible)?,
        Op::Alu(op) => step_op_alu(op, &mut vm.stack)
            .map(|_| None)
            .map_err(OpError::from_infallible)?,
        Op::Crypto(op) => step_op_crypto(op, &mut vm.stack)
            .map(|_| None)
            .map_err(OpError::from_infallible)?,
        Op::ParentMemory(op) => step_op_parent_memory(op, &mut vm.stack, &vm.parent_memory)
            .map(|_| None)
            .map_err(OpError::from_infallible)?,
        Op::Pred(op) => step_op_pred(op, &mut vm.stack)
            .map(|_| None)
            .map_err(OpError::from_infallible)?,
        Op::Stack(op) => step_op_stack(op, vm.pc, &mut vm.stack, &mut vm.repeat)
            .map_err(OpError::from_infallible)?,
        Op::TotalControlFlow(op) => step_op_total_control_flow(op, &mut vm.stack, vm.pc)
            .map_err(OpError::from_infallible)?,
        Op::Memory(op) => step_op_memory(op, &mut vm.stack, &mut vm.memory)
            .map(|_| None)
            .map_err(OpError::from_infallible)?,
        Op::StateRead(op) => step_op_state_reads(
            op,
            &access.this_solution().predicate_to_solve.contract,
            state,
            &mut vm.stack,
            &mut vm.memory,
        )
        .map(|_| None)?,
        Op::Compute(op) => step_op_compute(
            op,
            ComputeInputs {
                pc: vm.pc,
                stack: &mut vm.stack,
                memory: &mut vm.memory,
                parent_memory: vm.parent_memory.clone(),
                halt: vm.halt,
                repeat: &vm.repeat,
                cache: vm.cache.clone(),
                access,
                state_reads: state,
                op_access,
                op_gas_cost,
                gas_limit,
            },
        )
        .map(Some)?,
    };

    Ok(r)
}

/// Step forward execution by the given state reads operation.
pub fn step_op_state_reads<S>(
    op: asm::StateRead,
    contract_addr: &ContentAddress,
    state: &S,
    stack: &mut Stack,
    memory: &mut Memory,
) -> OpResult<(), S::Error>
where
    S: StateReads,
{
    match op {
        asm::StateRead::KeyRange => {
            crate::state_read::key_range(state.pre(), contract_addr, stack, memory)
        }
        asm::StateRead::KeyRangeExtern => {
            crate::state_read::key_range_ext(state.pre(), stack, memory)
        }
        essential_asm::StateRead::PostKeyRange => {
            crate::state_read::key_range(state.post(), contract_addr, stack, memory)
        }
        essential_asm::StateRead::PostKeyRangeExtern => {
            crate::state_read::key_range_ext(state.post(), stack, memory)
        }
    }
}

/// Step forward execution by the given compute operation.
pub fn step_op_compute<S, OA, OG>(
    op: asm::Compute,
    inputs: ComputeInputs<S, OA, OG>,
) -> OpResult<ProgramControlFlow, S::Error>
where
    S: StateReads,
    OA: OpAccess<Op = Op>,
    OA::Error: Into<OpError<S::Error>>,
    OG: OpGasCost,
{
    match op {
        asm::Compute::Compute => {
            crate::compute::compute(inputs).map(ProgramControlFlow::ComputeResult)
        }
        asm::Compute::ComputeEnd => Ok(ProgramControlFlow::ComputeEnd),
    }
}

/// Step forward execution by the given access operation.
pub fn step_op_access(
    access: Access,
    op: asm::Access,
    stack: &mut Stack,
    repeat: &mut Repeat,
    cache: &LazyCache,
) -> OpResult<()> {
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
        asm::Access::ThisAddress => access::this_address(access.this_solution(), stack),
        asm::Access::ThisContractAddress => {
            access::this_contract_address(access.this_solution(), stack)
        }
        asm::Access::RepeatCounter => access::repeat_counter(stack, repeat),
        asm::Access::PredicateExists => access::predicate_exists(stack, access.solutions, cache),
    }
}

/// Step forward execution by the given ALU operation.
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

/// Step forward execution by the given crypto operation.
pub fn step_op_crypto(op: asm::Crypto, stack: &mut Stack) -> OpResult<()> {
    match op {
        asm::Crypto::Sha256 => crypto::sha256(stack),
        asm::Crypto::VerifyEd25519 => crypto::verify_ed25519(stack),
        asm::Crypto::RecoverSecp256k1 => crypto::recover_secp256k1(stack),
    }
}

/// Step forward execution by the given predicate operation.
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

/// Step forward execution by the given stack operation.
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
        asm::Stack::Drop => stack.pop_len_words(|_| Ok(())),
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
) -> OpResult<Option<ProgramControlFlow>> {
    match op {
        asm::TotalControlFlow::JumpIf => total_control_flow::jump_if(stack, pc),
        asm::TotalControlFlow::HaltIf => total_control_flow::halt_if(stack),
        asm::TotalControlFlow::Halt => Ok(Some(ProgramControlFlow::Halt)),
        asm::TotalControlFlow::PanicIf => total_control_flow::panic_if(stack).map(|_| None),
    }
}

/// Step forward execution by the given memory operation.
pub fn step_op_memory(op: asm::Memory, stack: &mut Stack, memory: &mut Memory) -> OpResult<()> {
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
                Ok::<_, OpError>(())
            })?;
            Ok(())
        }
    }
}

/// Step forward execution by the given parent memory operation.
pub fn step_op_parent_memory(
    op: asm::ParentMemory,
    stack: &mut Stack,
    parent_memory: &[Arc<Memory>],
) -> OpResult<()> {
    let Some(memory) = parent_memory.last() else {
        return Err(ParentMemoryError::NoParent.into());
    };
    match op {
        asm::ParentMemory::Load => stack.pop1_push1(|addr| {
            let w = memory.load(addr)?;
            Ok(w)
        }),
        asm::ParentMemory::LoadRange => {
            let [addr, size] = stack.pop2()?;
            let words = memory.load_range(addr, size)?;
            Ok(stack.extend(words)?)
        }
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use crate::{
        types::{solution::Solution, ContentAddress, PredicateAddress},
        *,
    };
    use std::sync::Arc;

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

    pub(crate) fn test_solutions() -> Arc<Vec<Solution>> {
        Arc::new(vec![TEST_SOLUTION])
    }

    pub(crate) fn test_access() -> &'static Access {
        static INSTANCE: std::sync::LazyLock<Access> = std::sync::LazyLock::new(|| Access {
            solutions: test_solutions(),
            index: 0,
        });
        &INSTANCE
    }
}
