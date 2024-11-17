use crate::{
    error::OpError, future, Access, BytecodeMapped, BytecodeMappedLazy, Gas, GasLimit, LazyCache,
    Memory, Op, OpAccess, OpGasCost, Repeat, Stack, StateRead, StateReadError,
};

/// The operation execution state of the State Read VM.
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
    pub async fn exec_bytecode<'a, S, B>(
        &mut self,
        bytecode_mapped: &BytecodeMapped<B>,
        access: Access<'a>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
        B: core::ops::Deref<Target = [u8]>,
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
        let bytecode_lazy = BytecodeMappedLazy::new(bytecode_iter);
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
        OA: OpAccess<Op = Op> + Unpin,
        OA::Error: Into<OpError<S::Error>>,
    {
        future::exec(self, access, state_read, op_access, op_gas_cost, gas_limit).await
    }
}
