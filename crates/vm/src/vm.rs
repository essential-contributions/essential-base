//! The VM state machine, used to drive forward execution.

use crate::{
    error::{ExecError, OpError, OutOfGasError},
    sync::step_op,
    Access, BytecodeMapped, BytecodeMappedLazy, Gas, GasLimit, LazyCache, Memory, Op, OpAccess,
    OpGasCost, ProgramControlFlow, Repeat, Stack, StateRead,
};

/// The operation execution state of the VM.
#[derive(Debug, Default, PartialEq)]
pub struct Vm {
    /// The program counter, i.e. index of the current operation within the program.
    pub pc: usize,
    /// The stack machine.
    pub stack: Stack,
    /// The memory for temporary storage of words.
    pub memory: Memory,
    /// The repeat stack.
    pub repeat: Repeat,
    /// Lazily cached data for the VM.
    pub cache: LazyCache,
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
    /// or [`Vm::exec_bytecode_iter`] methods which allow for providing a more
    /// compact representation of the operations in the form of mapped bytecode.
    pub fn exec_ops<S>(
        &mut self,
        ops: &[Op],
        access: Access<'_>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, ExecError<S::Error>>
    where
        S: StateRead,
    {
        self.exec(access, state_read, ops, op_gas_cost, gas_limit)
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
        access: Access<'_>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, ExecError<S::Error>>
    where
        S: StateRead,
        B: core::ops::Deref<Target = [u8]>,
    {
        self.exec(access, state_read, bytecode_mapped, op_gas_cost, gas_limit)
    }

    /// Execute the given bytecode from the current state of the VM.
    ///
    /// This function uses synchronous state reading and is intended for use
    /// with in-memory state implementations.
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
    pub fn exec_bytecode_iter<S, I>(
        &mut self,
        bytecode_iter: I,
        access: Access<'_>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, ExecError<S::Error>>
    where
        S: StateRead,
        I: IntoIterator<Item = u8>,
        I::IntoIter: Unpin,
    {
        let bytecode_lazy = BytecodeMappedLazy::new(bytecode_iter);
        self.exec(access, state_read, bytecode_lazy, op_gas_cost, gas_limit)
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
        access: Access<'_>,
        state_read: &S,
        mut op_access: OA,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, ExecError<S::Error>>
    where
        S: StateRead,
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
            let res = step_op(access, op, self, state_read);

            #[cfg(feature = "tracing")]
            crate::trace_op_res(
                &mut op_access,
                self.pc,
                &self.stack,
                &self.memory,
                res.as_ref(),
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
                None => self.pc += 1,
            }
        }
        Ok(gas_spent)
    }
}
