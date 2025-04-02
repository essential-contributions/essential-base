use crate::{
    error::{ComputeError, ExecError, MemoryError, OpError, OpResult},
    Access, Gas, GasLimit, LazyCache, Memory, Op, OpAccess, OpGasCost, Repeat, Stack, StateReads,
    Vm,
};
use rayon::prelude::*;
use std::sync::Arc;

/// The limit on compute recursion depth.
pub const MAX_COMPUTE_DEPTH: usize = 1;

/// Inputs for the compute operation execution.
pub struct ComputeInputs<'a, S, OA, OG> {
    /// Parent VM program counter.
    pub pc: usize,
    /// Parent VM stack. Cloned for compute programs.
    pub stack: &'a mut Stack,
    /// Parent VM memory.
    /// At the beginning of compute operation, is pushed to `parent_memory`.
    /// At the end of compute operation, contains the memory resulting from compute threads.
    pub memory: &'a mut Memory,
    /// Read-only memory that is read by the compute threads.
    pub parent_memory: Vec<Arc<Memory>>,
    /// Repeat stack. Cloned for compute programs.
    pub repeat: &'a Repeat,
    /// Lazily cached data.
    pub cache: Arc<LazyCache>,
    /// [`Access`] required for VM execution. Cloned for compute programs.
    pub access: Access,
    /// [`StateReads`] for VM execution.
    pub state_reads: &'a S,
    /// [`OpAccess`] for VM execution.
    pub op_access: OA,
    /// [`OpGasCost`] for VM execution.
    pub op_gas_cost: &'a OG,
    /// [`GasLimit`] for VM execution.
    pub gas_limit: GasLimit,
}

/// The Compute op implementation.
///
/// Computes as many programs as is the input to this op in parallel.
/// Each compute program executes on a constructed VM that has
/// - Read-only access to parent VM memory
/// - Parent VM stack with an additional value on top that is the compute index
/// - Parent program counter + 1
///
/// When compute program returns after seeing op ComputeEnd or when ops come to an end,
/// parent VM memory is updated to be the concatenation of children's memories.
///
/// Intended usage is parallelizable computation that can be distributed to self index-aware compute programs.
///
/// Returns resulting program counter and spent gas.
pub fn compute<S, OA, OG>(inputs: ComputeInputs<S, OA, OG>) -> OpResult<(usize, Gas), S::Error>
where
    S: StateReads,
    OA: OpAccess<Op = Op>,
    OA::Error: Into<OpError<S::Error>>,
    OG: OpGasCost,
{
    let ComputeInputs {
        pc,
        stack,
        memory,
        mut parent_memory,
        repeat,
        cache,
        access,
        state_reads,
        op_access,
        op_gas_cost,
        gas_limit,
    } = inputs;

    // Pop the number of compute threads to spawn.
    let compute_breadth = stack
        .pop()
        .map_err(|e| OpError::Compute(ComputeError::Stack(e)))?;
    if compute_breadth < 1 {
        return Err(OpError::Compute(ComputeError::<S::Error>::InvalidBreadth(
            compute_breadth,
        )));
    }

    // Append parent memory to be read by spawned threads.
    if parent_memory.len() < MAX_COMPUTE_DEPTH {
        parent_memory.push(Arc::new(memory.to_owned()));
    } else {
        return Err(ComputeError::DepthReached(MAX_COMPUTE_DEPTH).into());
    }

    // Compute in parallel.
    let results: Result<Vec<(Gas, usize, Memory)>, _> = (0..compute_breadth)
        .into_par_iter()
        .map(|compute_index| {
            // Clone stack and push compute program index.
            let mut stack = stack.clone();
            stack
                .push(compute_index)
                .map_err(|e| ExecError(pc, OpError::Compute(e.into())))?;

            let mut vm = Vm {
                pc: pc + 1,
                stack,
                memory: Memory::new(),
                parent_memory: parent_memory.clone(),
                repeat: repeat.clone(),
                cache: cache.clone(),
            };

            // Execute child VM.
            vm.exec(
                access.clone(),
                state_reads,
                op_access.clone(),
                op_gas_cost,
                gas_limit,
            )
            .map(|gas| (gas, vm.pc, vm.memory))
        })
        .collect();

    let oks = results.map_err(|e| OpError::Compute(ComputeError::Exec(Box::new(e))))?;

    // Process compute program results.
    let (pc, total_gas) = compute_effects(memory, pc, oks)?;

    Ok((pc, total_gas))
}

// Allocates the resulting memories from compute programs to the parent VM memory.
// Updates parent VM program counter to the largest pc returned from the compute programs.
//
// Returns maximum program counter and total gas spent in compute programs.
fn compute_effects(
    memory: &mut Memory,
    mut pc: usize,
    compute_results: Vec<(Gas, usize, Memory)>,
) -> Result<(usize, Gas), MemoryError> {
    let mut total_gas = 0;

    let mut memory_to_alloc = 0;
    compute_results
        .iter()
        .for_each(|(_, _, mem)| memory_to_alloc += mem.len().unwrap_or_default());
    // moving pointer to index in parent memory to store new values at
    let mut memory_pointer = memory.len().expect("memory has to have length");
    // allocate enough space in the parent memory at once
    memory.alloc(memory_to_alloc)?;
    // concat compute memories to parent memory one by one
    compute_results.iter().for_each(|(gas, c_pc, mem)| {
        pc = std::cmp::max(pc, *c_pc);
        total_gas += gas;
        memory.store_range(memory_pointer, mem).expect("for now");
        memory_pointer += mem.len().unwrap();
    });

    Ok((pc, total_gas))
}
