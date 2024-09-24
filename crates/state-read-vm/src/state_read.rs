//! State read operation implementations.

use crate::{
    error::{OpAsyncError, StackError},
    OpAsyncResult, Vm,
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use essential_types::{convert::u8_32_from_word_4, ContentAddress, Key, Word};

/// Access to state required by the state read VM.
pub trait StateRead {
    /// An error type describing any cases that might occur during state reading.
    type Error: core::fmt::Debug + core::fmt::Display;
    /// The future type returned from the `key_range` method.
    ///
    /// ## Unpin
    ///
    /// This `Future` must be `Unpin` in order for the `Vm`'s `ExecFuture`
    /// to remain zero-allocation by default. Implementers may decide on
    /// whether they require dynamic allocation as a part of their `StateRead`
    /// implementation.
    ///
    /// It is likely that in-memory implementations may be `Unpin` by default
    /// using `std::future::Ready`, however more involved implementations that
    /// require calling `async` functions with anonymised return types may
    /// require using a `Box` in order to name the anonymised type.
    type Future: Future<Output = Result<Vec<Vec<Word>>, Self::Error>> + Unpin;

    /// Read the given number of values from state at the given key associated
    /// with the given contract address.
    fn key_range(&self, contract_addr: ContentAddress, key: Key, num_values: usize)
        -> Self::Future;
}

/// A future representing the asynchronous `StateRead` (or `StateReadExtern`) operation.
///
/// Performs the state read and then writes the result to memory.
pub(crate) struct StateReadFuture<'vm, S>
where
    S: StateRead,
{
    /// The future produced by the `StateRead::key_range` implementation.
    future: S::Future,
    /// The index of the slot that this should start writing into.
    slot_index: usize,
    /// Access to the `Vm` so that the result of the future can be written to memory.
    pub(crate) vm: &'vm mut Vm,
}

impl<'vm, S> Future for StateReadFuture<'vm, S>
where
    S: StateRead,
{
    /// Returns a `Result` representing whether or not the state was read and
    /// written to memory successfully.
    type Output = OpAsyncResult<(), S::Error>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match Pin::new(&mut self.future).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => {
                let slot_index = self.slot_index;
                let res = res
                    .map_err(OpAsyncError::StateRead)
                    .and_then(|words| write_values_to_state_slots(self.vm, slot_index, words));
                Poll::Ready(res)
            }
        }
    }
}

/// `StateRead::KeyRange` operation.
pub fn key_range<'vm, S>(
    state_read: &S,
    contract_addr: &ContentAddress,
    vm: &'vm mut Vm,
) -> OpAsyncResult<StateReadFuture<'vm, S>, S::Error>
where
    S: StateRead,
{
    let slot_index = vm.stack.pop()?;
    let slot_index = usize::try_from(slot_index).map_err(|_| StackError::IndexOutOfBounds)?;
    let future = read_key_range(state_read, contract_addr, vm)?;
    Ok(StateReadFuture {
        future,
        slot_index,
        vm,
    })
}

/// `StateRead::KeyRangeExtern` operation.
pub fn key_range_ext<'vm, S>(
    state_read: &S,
    vm: &'vm mut Vm,
) -> OpAsyncResult<StateReadFuture<'vm, S>, S::Error>
where
    S: StateRead,
{
    let slot_index = vm.stack.pop()?;
    let slot_index = usize::try_from(slot_index).map_err(|_| StackError::IndexOutOfBounds)?;
    let future = read_key_range_ext(state_read, vm)?;
    Ok(StateReadFuture {
        future,
        slot_index,
        vm,
    })
}

/// Read the length and key from the top of the stack and read the associated words from state.
fn read_key_range<S>(
    state_read: &S,
    contract_addr: &ContentAddress,
    vm: &mut Vm,
) -> OpAsyncResult<S::Future, S::Error>
where
    S: StateRead,
{
    let num_keys = vm.stack.pop()?;
    let num_keys = usize::try_from(num_keys).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = vm
        .stack
        .pop_len_words::<_, _, StackError>(|words| Ok(words.to_vec()))?;
    Ok(state_read.key_range(contract_addr.clone(), key, num_keys))
}

/// Read the length, key and external contract address from the top of the stack and
/// read the associated words from state.
fn read_key_range_ext<S>(state_read: &S, vm: &mut Vm) -> OpAsyncResult<S::Future, S::Error>
where
    S: StateRead,
{
    let num_keys = vm.stack.pop()?;
    let num_keys = usize::try_from(num_keys).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = vm
        .stack
        .pop_len_words::<_, _, StackError>(|words| Ok(words.to_vec()))?;
    let contract_addr = ContentAddress(u8_32_from_word_4(vm.stack.pop4()?));
    Ok(state_read.key_range(contract_addr, key, num_keys))
}

/// Write the given values to mutable state slots.
fn write_values_to_state_slots<E>(
    vm: &mut Vm,
    slot_index: usize,
    values: Vec<Vec<Word>>,
) -> OpAsyncResult<(), E> {
    vm.state_slots_mut.store_slots_range(slot_index, values)?;
    Ok(())
}
