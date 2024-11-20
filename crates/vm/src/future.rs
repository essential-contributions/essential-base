use crate::{
    asm::Op,
    error::{OpAsyncError, OpError, OutOfGasError, StateReadError},
    state_read::{self, StateReadFuture},
    step_op_sync, Access, ContentAddress, Gas, GasLimit, OpAccess, OpAsync, OpAsyncResult,
    OpGasCost, OpKind, StateRead, Vm,
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// A future that when polled attempts to make progress on VM execution.
///
/// This poll implementation steps forward the VM by the stored operations,
/// handling synchronous and asynchronous operations differently:
///
/// - For synchronous operations, it directly steps the VM to execute the
///   operation.
/// - For asynchronous operations, it creates a future that will complete
///   the operation and temporarily takes ownership of the VM. This future
///   is stored in `pending_op` until it's ready.
///
/// This type should not be constructed directly. Instead, it is used as a part
/// of the implementation of [`Vm::exec`] and exposed publicly for documentation
/// of its behaviour.
///
/// ## Yield Behavior
///
/// Execution yields in two scenarios:
///
/// - **Asynchronous Operations**: When an async operation is encountered,
///   the method yields until the operation's future is ready. This allows
///   other tasks to run while awaiting the asynchronous operation to
///   complete.
/// - **Gas Yield Limit Reached**: The method also yields based on a gas
///   spending limit. If executing an operation causes `gas.spent` to exceed
///   `gas.next_yield_threshold`, the method yields to allow the scheduler
///   to run other tasks. This prevents long or complex sequences of
///   operations from monopolizing CPU time.
///
/// Upon yielding, the method ensures that the state of the VM and the
/// execution context (including gas counters and any pending operations)
/// are preserved for when the `poll` method is called again.
///
/// ## Error Handling
///
/// Errors encountered during operation execution result in an immediate
/// return of `Poll::Ready(Err(...))`, encapsulating the error within a
/// `StateReadError`. This includes errors from:
///
/// - Synchronous operations that fail during their execution.
/// - Asynchronous operations, where errors are handled once the future
///   resolves.
///
/// The VM's program counter will remain on the operation that caused the
/// error.
///
/// ## Completion
///
/// The future completes (`Poll::Ready(Ok(...))`) when all operations have
/// been executed and no more work remains. At this point, ownership over
/// the VM is dropped and the total amount of gas spent during execution is
/// returned. Attempting to poll the future after completion will panic.
pub struct ExecFuture<'a, S, OA, OG>
where
    S: StateRead,
{
    /// Access to solution data.
    access: Access<'a>,
    /// Access to state reading.
    state_read: &'a S,
    /// Access to operations.
    op_access: OA,
    /// A function that, given a reference to an op, returns its gas cost.
    op_gas_cost: &'a OG,
    /// Store the VM in an `Option` so that we can `take` it upon future completion.
    vm: Option<&'a mut Vm>,
    /// Gas spent during execution so far.
    gas: GasExec,
    /// In the case that the operation future is pending (i.e a state read is in
    /// progress), we store the future here.
    pending_op: Option<PendingOp<'a, S>>,
}

/// Track gas limits and expenditure for execution.
struct GasExec {
    /// The total and yield gas limits.
    limit: GasLimit,
    /// The gas threshold at which the future should yield.
    next_yield_threshold: Gas,
    /// The total gas limit.
    spent: Gas,
}

/// Encapsulates a pending operation.
struct PendingOp<'a, S>
where
    S: StateRead,
{
    // The future representing the operation in progress.
    future: StepOpAsyncFuture<'a, S>,
    /// Total gas that will have been spent upon completing the op.
    next_spent: Gas,
}

/// The future type produced when performing an async operation.
enum StepOpAsyncFuture<'a, S>
where
    S: StateRead,
{
    /// The async `StateRead::WordRange` (or `WordRangeExtern`) operation future.
    StateRead(StateReadFuture<'a, S>),
}

impl From<GasLimit> for GasExec {
    /// Initialise gas execution tracking from a given gas limit.
    fn from(limit: GasLimit) -> Self {
        GasExec {
            spent: 0,
            next_yield_threshold: limit.per_yield,
            limit,
        }
    }
}

// Allow for consuming the async operation future to retake ownership of the stored `&mut Vm`.
impl<'a, S> From<StepOpAsyncFuture<'a, S>> for &'a mut Vm
where
    S: StateRead,
{
    fn from(future: StepOpAsyncFuture<'a, S>) -> Self {
        match future {
            StepOpAsyncFuture::StateRead(future) => future.vm,
        }
    }
}

impl<'a, S, OA, OG> Future for ExecFuture<'a, S, OA, OG>
where
    S: StateRead,
    OA: OpAccess<Op = Op> + Unpin,
    OG: OpGasCost,
    OA::Error: Into<OpError<S::Error>>,
{
    /// Returns a result with the total gas spent.
    type Output = Result<Gas, StateReadError<S::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        // Poll the async operation future if there is one pending.
        let vm = match self.pending_op.as_mut() {
            None => self.vm.take().expect("future polled after completion"),
            Some(pending) => {
                let res = match Pin::new(&mut pending.future).poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(ready) => ready,
                };

                // Drop the future now we've resumed, retake ownership of the `&mut Vm`.
                let pending = self.pending_op.take().expect("guaranteed `Some`");
                let next_spent = pending.next_spent;
                let vm: &'a mut Vm = pending.future.into();

                // Handle the op result.
                #[cfg(feature = "tracing")]
                crate::trace_op_res(
                    &mut self.op_access,
                    vm.pc,
                    &vm.stack,
                    &vm.memory,
                    res.as_ref(),
                );

                match res {
                    Ok(new_pc) => vm.pc = new_pc,
                    Err(err) => {
                        let err = StateReadError::Op(vm.pc, err.into());
                        return Poll::Ready(Err(err));
                    }
                };

                // Update gas spent and threshold now that we've resumed.
                self.gas.spent = next_spent;
                self.gas.next_yield_threshold =
                    self.gas.spent.saturating_add(self.gas.limit.per_yield);
                vm
            }
        };

        // Step forward the virtual machine by the next operation.
        while let Some(res) = self.op_access.op_access(vm.pc) {
            // Handle any potential operation access error.
            let op = match res {
                Ok(op) => op,
                Err(err) => {
                    let err = StateReadError::Op(vm.pc, err.into());
                    return Poll::Ready(Err(err));
                }
            };

            let op_gas = self.op_gas_cost.op_gas_cost(&op);

            // Check that the operation wouldn't exceed gas limit.
            let next_spent = match self
                .gas
                .spent
                .checked_add(op_gas)
                .filter(|&spent| spent <= self.gas.limit.total)
                .ok_or_else(|| out_of_gas(&self.gas, op_gas))
                .map_err(|err| StateReadError::Op(vm.pc, err.into()))
            {
                Err(err) => return Poll::Ready(Err(err)),
                Ok(next_spent) => next_spent,
            };

            let res = match OpKind::from(op) {
                OpKind::Sync(op) => step_op_sync(op, self.access, vm),
                OpKind::Async(op) => {
                    // Async op takes ownership of the VM and returns it upon future completion.
                    let contract_addr = self.access.this_data().predicate_to_solve.contract.clone();
                    let pc = vm.pc;
                    let future = match step_op_async(op, contract_addr, self.state_read, vm) {
                        Err(err) => {
                            let err = StateReadError::Op(pc, err.into());
                            return Poll::Ready(Err(err));
                        }
                        Ok(fut) => fut,
                    };
                    self.pending_op = Some(PendingOp { future, next_spent });
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
            };

            #[cfg(feature = "tracing")]
            crate::trace_op_res(
                &mut self.op_access,
                vm.pc,
                &vm.stack,
                &vm.memory,
                res.as_ref(),
            );

            // Handle any errors.
            let opt_new_pc = match res {
                Ok(opt) => opt,
                Err(err) => {
                    return Poll::Ready(Err(StateReadError::Op(vm.pc, err.into())));
                }
            };

            // Operation successful, so update gas spent.
            self.gas.spent = next_spent;

            // Update the program counter, or exit if we're done.
            match opt_new_pc {
                Some(new_pc) => vm.pc = new_pc,
                // `None` is returned after encountering a `Halt` operation.
                None => return Poll::Ready(Ok(self.gas.spent)),
            }

            // Yield if we've reached our gas limit.
            if self.gas.next_yield_threshold <= self.gas.spent {
                self.gas.next_yield_threshold =
                    self.gas.spent.saturating_add(self.gas.limit.per_yield);
                self.vm = Some(vm);
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        }

        Poll::Ready(Ok(self.gas.spent))
    }
}

impl<'vm, S> Future for StepOpAsyncFuture<'vm, S>
where
    S: StateRead,
{
    // Future returns a result with the new program counter.
    type Output = OpAsyncResult<usize, S::Error>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let (prev_pc, res) = match *self {
            Self::StateRead(ref mut future) => {
                let pc = future.vm.pc;
                match Pin::new(future).poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(res) => (pc, res),
                }
            }
        };
        // Every operation besides control flow steps forward program counter by 1.
        let new_pc = prev_pc.checked_add(1).ok_or(OpAsyncError::PcOverflow)?;
        let res = res.map(|()| new_pc);
        Poll::Ready(res)
    }
}

/// Creates the VM execution future.
pub(crate) fn exec<'a, S, OA, OG>(
    vm: &'a mut Vm,
    access: Access<'a>,
    state_read: &'a S,
    op_access: OA,
    op_gas_cost: &'a OG,
    gas_limit: GasLimit,
) -> ExecFuture<'a, S, OA, OG>
where
    S: StateRead,
    OA: OpAccess<Op = Op> + Unpin,
    OG: OpGasCost,
    OA::Error: Into<OpError<S::Error>>,
{
    ExecFuture {
        access,
        state_read,
        op_access,
        op_gas_cost,
        vm: Some(vm),
        gas: GasExec::from(gas_limit),
        pending_op: None,
    }
}

/// Step forward the given `Vm` with the given asynchronous operation.
///
/// Returns a future representing the completion of the operation.
fn step_op_async<'a, S>(
    op: OpAsync,
    contract_addr: ContentAddress,
    state_read: &'a S,
    vm: &'a mut Vm,
) -> OpAsyncResult<StepOpAsyncFuture<'a, S>, S::Error>
where
    S: StateRead,
{
    match op {
        OpAsync::StateReadKeyRange => {
            let future = state_read::key_range(state_read, &contract_addr, &mut *vm)?;
            Ok(StepOpAsyncFuture::StateRead(future))
        }
        OpAsync::StateReadKeyRangeExt => {
            let future = state_read::key_range_ext(state_read, &mut *vm)?;
            Ok(StepOpAsyncFuture::StateRead(future))
        }
    }
}

/// Shorthand for constructing an `OutOfGasError`.
fn out_of_gas(exec: &GasExec, op_gas: Gas) -> OutOfGasError {
    OutOfGasError {
        spent: exec.spent,
        limit: exec.limit.total,
        op_gas,
    }
}
