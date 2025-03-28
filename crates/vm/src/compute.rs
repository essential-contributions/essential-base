use crate::{
    error::{ComputeError, OpError, OpResult},
    Access, GasLimit, LazyCache, Memory, Op, OpAccess, OpGasCost, Repeat, Stack, StateReads, Vm,
};
use rayon::prelude::*;
use std::sync::Arc;

#[cfg(test)]
mod tests;

/// The limit on compute recursion depth.
pub const MAX_COMPUTE_DEPTH: usize = 1;

pub struct ComputeInputs<'a, S, OA, OG> {
    pub stack: &'a mut Stack,
    pub memory: &'a mut Memory,
    pub parent_memory: Vec<Arc<Memory>>,
    pub repeat: &'a Repeat,
    pub cache: Arc<LazyCache>,
    pub access: Access,
    pub state_read: &'a S,
    pub op_access: OA,
    pub op_gas_cost: &'a OG,
    pub gas_limit: GasLimit,
}

/// The Compute op implementation.
///
/// Pops the number of compute threads from the stack.
pub fn compute<S, OA, OG>(inputs: ComputeInputs<S, OA, OG>) -> OpResult<()>
where
    S: StateReads,
    OA: OpAccess<Op = Op>,
    OA::Error: Into<OpError<S::Error>>,
    OG: OpGasCost,
{
    let ComputeInputs {
        stack,
        memory,
        mut parent_memory,
        repeat,
        cache,
        access,
        state_read,
        op_access,
        op_gas_cost,
        gas_limit,
    } = inputs;

    let compute_breadth = stack.pop()?;

    if parent_memory.len() < MAX_COMPUTE_DEPTH {
        parent_memory.push(Arc::new(memory.to_owned()));
    } else {
        return Err(ComputeError::DepthReached(MAX_COMPUTE_DEPTH).into());
    }

    let results = (0..compute_breadth).into_par_iter().map(|compute_index| {
        let mut stack = stack.clone();
        let res = stack.push(compute_index);
        let mut vm = Vm {
            stack,
            memory: Memory::new(),
            parent_memory: parent_memory.clone(),
            repeat: repeat.clone(),
            cache: cache.clone(),
            ..Default::default()
        };

        vm.exec(
            access.clone(),
            state_read,
            op_access.clone(),
            op_gas_cost,
            gas_limit,
        )
    });
    Ok(())
}

/// The ComputeEnd op implementation.
pub fn compute_end() -> OpResult<()> {
    todo!("to be implemented: ComputeEnd");
}
