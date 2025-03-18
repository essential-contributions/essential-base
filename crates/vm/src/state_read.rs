//! State read operation implementations.

use crate::{
    error::{
        MemoryError, OpAsyncError, OpAsyncResult, OpStateSyncError, OpStateSyncResult, StackError,
        StateReadArgError,
    },
    Memory, Stack, Vm,
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use essential_types::{convert::u8_32_from_word_4, ContentAddress, Key, Value, Word};

#[cfg(test)]
mod tests;

/// Read-only access to state required by the VM.
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

/// Read-only access to state required by the VM.
pub trait StateReadSync {
    /// An error type describing any cases that might occur during state reading.
    type Error: core::fmt::Debug + core::fmt::Display;

    /// Read the given number of values from state at the given key associated
    /// with the given contract address.
    fn key_range(
        &self,
        contract_addr: ContentAddress,
        key: Key,
        num_values: usize,
    ) -> Result<Vec<Vec<Word>>, Self::Error>;
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
    /// The memory address at which this should start writing into.
    mem_addr: usize,
    /// Access to the `Vm` so that the result of the future can be written to memory.
    pub(crate) vm: &'vm mut Vm,
}

impl<S> Future for StateReadFuture<'_, S>
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
                let mem_addr = self.mem_addr;
                let res = res.map_err(OpAsyncError::StateRead).and_then(|words| {
                    Ok(write_values_to_memory(
                        mem_addr,
                        words,
                        &mut self.vm.memory,
                    )?)
                });
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
    let mem_addr = pop_memory_address(&mut vm.stack)?;
    let future = read_key_range(state_read, contract_addr, vm)?;
    Ok(StateReadFuture {
        future,
        mem_addr,
        vm,
    })
}

/// `StateRead::KeyRange` operation.
/// Uses a synchronous state read.
pub fn key_range_sync<S>(
    state_read: &S,
    contract_addr: &ContentAddress,
    stack: &mut Stack,
    memory: &mut Memory,
) -> OpStateSyncResult<(), S::Error>
where
    S: StateReadSync,
{
    let mem_addr = pop_memory_address(stack)?;
    let values = read_key_range_sync(state_read, contract_addr, stack)?;
    write_values_to_memory(mem_addr, values, memory)?;
    Ok(())
}

/// `StateRead::KeyRangeExtern` operation.
pub fn key_range_ext<'vm, S>(
    state_read: &S,
    vm: &'vm mut Vm,
) -> OpAsyncResult<StateReadFuture<'vm, S>, S::Error>
where
    S: StateRead,
{
    let mem_addr = pop_memory_address(&mut vm.stack)?;
    let future = read_key_range_ext(state_read, vm)?;
    Ok(StateReadFuture {
        future,
        mem_addr,
        vm,
    })
}

/// `StateRead::KeyRangeExtern` operation.
/// Uses a synchronous state read.
pub fn key_range_ext_sync<S>(
    state_read: &S,
    stack: &mut Stack,
    memory: &mut Memory,
) -> OpStateSyncResult<(), S::Error>
where
    S: StateReadSync,
{
    let mem_addr = pop_memory_address(stack)?;
    let values = read_key_range_ext_sync(state_read, stack)?;
    write_values_to_memory(mem_addr, values, memory)?;
    Ok(())
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
    let (key, num_keys) = pop_key_range_args(&mut vm.stack)?;
    Ok(state_read.key_range(contract_addr.clone(), key, num_keys))
}

/// Read the length and key from the top of the stack and read the associated words from state.
/// Uses a synchronous state read.
fn read_key_range_sync<S>(
    state_read: &S,
    contract_addr: &ContentAddress,
    stack: &mut Stack,
) -> OpStateSyncResult<Vec<Value>, S::Error>
where
    S: StateReadSync,
{
    let (key, num_keys) = pop_key_range_args(stack)?;
    state_read
        .key_range(contract_addr.clone(), key, num_keys)
        .map_err(OpStateSyncError::StateRead)
}

/// Read the length, key and external contract address from the top of the stack and
/// read the associated words from state.
fn read_key_range_ext<S>(state_read: &S, vm: &mut Vm) -> OpAsyncResult<S::Future, S::Error>
where
    S: StateRead,
{
    let (key, num_keys) = pop_key_range_args(&mut vm.stack)?;
    let contract_addr = ContentAddress(u8_32_from_word_4(vm.stack.pop4()?));
    Ok(state_read.key_range(contract_addr, key, num_keys))
}

/// Read the length, key and external contract address from the top of the stack and
/// read the associated words from state.
/// Uses a synchronous state read.
fn read_key_range_ext_sync<S>(
    state_read: &S,
    stack: &mut Stack,
) -> OpStateSyncResult<Vec<Value>, S::Error>
where
    S: StateReadSync,
{
    let (key, num_keys) = pop_key_range_args(stack)?;
    let contract_addr = ContentAddress(u8_32_from_word_4(stack.pop4()?));
    state_read
        .key_range(contract_addr, key, num_keys)
        .map_err(OpStateSyncError::StateRead)
}

/// Pop the memory address that the state read will write to from the stack.
fn pop_memory_address(stack: &mut Stack) -> Result<usize, StateReadArgError> {
    let mem_addr = stack.pop()?;
    let mem_addr = usize::try_from(mem_addr).map_err(|_| MemoryError::IndexOutOfBounds)?;
    Ok(mem_addr)
}

/// Pop the key and number of keys from the stack.
fn pop_key_range_args(stack: &mut Stack) -> Result<(Key, usize), StackError> {
    let num_keys = stack.pop()?;
    let num_keys = usize::try_from(num_keys).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = stack.pop_len_words::<_, _, StackError>(|words| Ok(words.to_vec()))?;
    Ok((key, num_keys))
}

/// Write the given values to memory.
fn write_values_to_memory(
    mem_addr: usize,
    values: Vec<Vec<Word>>,
    memory: &mut Memory,
) -> Result<(), MemoryError> {
    let values_len = Word::try_from(values.len()).map_err(|_| MemoryError::Overflow)?;
    let index_len_pairs_len = values_len.checked_mul(2).ok_or(MemoryError::Overflow)?;
    let mut mem_addr = Word::try_from(mem_addr).map_err(|_| MemoryError::IndexOutOfBounds)?;
    let mut value_addr = mem_addr
        .checked_add(index_len_pairs_len)
        .ok_or(MemoryError::Overflow)?;
    for value in values {
        let value_len = Word::try_from(value.len()).map_err(|_| MemoryError::Overflow)?;
        // Write the [index, len] pair.
        memory.store_range(mem_addr, &[value_addr, value_len])?;
        // Write the value.
        memory.store_range(value_addr, &value)?;
        // No need to check addition here as `store_range` would have failed.
        value_addr += value_len;
        mem_addr += 2;
    }
    Ok(())
}
