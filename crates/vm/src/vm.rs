//! The VM state machine, used to drive forward execution.

use crate::{
    error::{EvalError, EvalResult, ExecError, OpError, OutOfGasError},
    sync::step_op,
    Access, BytecodeMapped, Gas, GasLimit, LazyCache, Memory, Op, OpAccess, OpGasCost,
    ProgramControlFlow, Repeat, Stack, StateReads,
};
use essential_types::convert::bool_from_word;
use std::sync::Arc;

/// The operation execution state of the VM.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vm {
    /// The program counter, i.e. index of the current operation within the program.
    pub pc: usize,
    /// The stack machine.
    pub stack: Stack,
    /// The memory for temporary storage of words.
    pub memory: Memory,
    /// The stack of parent `Memory`s.
    ///
    /// This is empty at the beginning of execution, but is pushed to each time
    /// we enter a [`Compute`] op context with the parent's `Memory`.
    ///
    /// This can also be used to observe the `Compute` op depth.
    pub parent_memory: Vec<Arc<Memory>>,
    /// The repeat stack.
    pub repeat: Repeat,
    /// Lazily cached data for the VM.
    pub cache: Arc<LazyCache>,
}

impl Vm {
    /// Execute the given operations from the current state of the VM.
    ///
    /// This function uses synchronous state reading and is intended for use
    /// with in-memory state implementations.
    ///
    /// Upon reaching a `Halt` operation or reaching the end of the operation
    /// sequence, returns the gas spent and the `Vm` will be left in the
    /// resulting state.
    ///
    /// This is a wrapper around [`Vm::exec`] that expects operation access in
    /// the form of a `&[Op]`.
    ///
    /// If memory bloat is a concern, consider using the [`Vm::exec_bytecode`]
    /// method which allows for providing a more compact representation of the
    /// operations in the form of mapped bytecode.
    pub fn exec_ops<S>(
        &mut self,
        ops: &[Op],
        access: Access,
        state_reads: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, ExecError<S::Error>>
    where
        S: StateReads,
    {
        self.exec(access, state_reads, ops, op_gas_cost, gas_limit)
    }

    /// Execute the given mapped bytecode from the current state of the VM.
    ///
    /// This function uses synchronous state reading and is intended for use
    /// with in-memory state implementations.
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
    pub fn exec_bytecode<S, B>(
        &mut self,
        bytecode_mapped: &BytecodeMapped<B>,
        access: Access,
        state_reads: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, ExecError<S::Error>>
    where
        S: StateReads,
        B: core::ops::Deref<Target = [u8]> + Send + Sync,
    {
        self.exec(access, state_reads, bytecode_mapped, op_gas_cost, gas_limit)
    }

    /// Execute the given operations synchronously from the current state of the VM.
    ///
    /// This function uses synchronous state reading and is intended for use
    /// with in-memory state implementations.
    ///
    /// Upon reaching a `Halt` operation or reaching the end of the operation
    /// sequence, returns the gas spent and the `Vm` will be left in the
    /// resulting state.
    pub fn exec<S, OA>(
        &mut self,
        access: Access,
        state_reads: &S,
        op_access: OA,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, ExecError<S::Error>>
    where
        S: StateReads,
        OA: OpAccess<Op = Op>,
        OA::Error: Into<OpError<S::Error>>,
    {
        // Track the gas spent.
        let mut gas_spent: u64 = 0;

        // Execute each operation
        while let Some(res) = op_access.op_access(self.pc) {
            let op = res.map_err(|err| ExecError(self.pc, err.into()))?;

            // Calculate the gas cost of the operation.
            let op_gas = op_gas_cost.op_gas_cost(&op);

            // Check that the operation wouldn't exceed gas limit.
            let next_spent = gas_spent
                .checked_add(op_gas)
                .filter(|&spent| spent <= gas_limit.total)
                .ok_or(ExecError(
                    self.pc,
                    OutOfGasError {
                        spent: gas_spent,
                        op_gas,
                        limit: gas_limit.total,
                    }
                    .into(),
                ))?;

            // Update the gas spent.
            gas_spent = next_spent;

            // Execute the operation.
            let res = step_op(
                access.clone(),
                op,
                self,
                state_reads,
                op_access.clone(),
                op_gas_cost,
                gas_limit,
            );

            #[cfg(feature = "tracing")]
            crate::trace_op_res(
                &op_access,
                self.pc,
                &self.stack,
                &self.memory,
                &self.parent_memory,
                &res,
            );

            // Handle the result of the operation.
            let update = match res {
                Ok(update) => update,
                Err(err) => return Err(ExecError(self.pc, err)),
            };

            // Update the program counter.
            match update {
                Some(ProgramControlFlow::Pc(new_pc)) => self.pc = new_pc,
                Some(ProgramControlFlow::Halt) => break,
                Some(ProgramControlFlow::ComputeEnd) => {
                    self.pc += 1;
                    break;
                }
                // TODO: compute gas_spent is not inferrable above
                Some(ProgramControlFlow::ComputeResult((pc, gas))) => {
                    gas_spent += gas;
                    self.pc = pc;
                }
                None => self.pc += 1,
            }
        }
        Ok(gas_spent)
    }

    /// Evaluate a slice of synchronous operations and return their boolean result.
    ///
    /// This is the same as [`exec_ops`], but retrieves the boolean result from the resulting stack.
    pub fn eval_ops<S>(
        &mut self,
        ops: &[Op],
        access: Access,
        state: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> EvalResult<bool, S::Error>
    where
        S: StateReads,
    {
        self.eval(ops, access, state, op_gas_cost, gas_limit)
    }

    // Evaluate the operations of a single synchronous program and return its boolean result.
    ///
    /// This is the same as [`exec`], but retrieves the boolean result from the resulting stack.
    pub fn eval<OA, S>(
        &mut self,
        op_access: OA,
        access: Access,
        state: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> EvalResult<bool, S::Error>
    where
        OA: OpAccess<Op = Op>,
        OA::Error: Into<OpError<S::Error>>,
        S: StateReads,
    {
        self.exec(access, state, op_access, op_gas_cost, gas_limit)?;

        let word = match self.stack.last() {
            Some(&w) => w,
            None => return Err(EvalError::InvalidEvaluation(self.stack.clone())),
        };
        bool_from_word(word).ok_or_else(|| EvalError::InvalidEvaluation(self.stack.clone()))
    }
}
