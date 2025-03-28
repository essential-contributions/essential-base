//! State read operation implementations.

use crate::{
    error::{MemoryError, OpError, OpResult, StackError, StateReadArgError},
    Memory, Stack,
};
use essential_types::{convert::u8_32_from_word_4, ContentAddress, Key, Value, Word};

#[cfg(test)]
mod tests;

/// Read-only access to state required by the VM.
pub trait StateRead: Send + Sync {
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

/// Pre and post sync state reads.
pub trait StateReads: Send + Sync {
    /// Common error type
    type Error: core::fmt::Debug + core::fmt::Display + Send;

    /// Pre state read
    type Pre: StateRead<Error = Self::Error>;

    /// Post state read
    type Post: StateRead<Error = Self::Error>;

    /// Get the pre state read
    fn pre(&self) -> &Self::Pre;

    /// Get the post state read
    fn post(&self) -> &Self::Post;
}

impl<S, P> StateReads for (S, P)
where
    S: StateRead,
    P: StateRead<Error = S::Error>,
    S::Error: Send,
{
    type Error = S::Error;

    type Pre = S;

    type Post = P;

    fn pre(&self) -> &Self::Pre {
        &self.0
    }

    fn post(&self) -> &Self::Post {
        &self.1
    }
}
/// `StateRead::KeyRange` operation.
/// Uses a synchronous state read.
pub fn key_range<S>(
    state_read: &S,
    contract_addr: &ContentAddress,
    stack: &mut Stack,
    memory: &mut Memory,
) -> OpResult<(), S::Error>
where
    S: StateRead,
{
    let mem_addr = pop_memory_address(stack)?;
    let values = read_key_range(state_read, contract_addr, stack)?;
    write_values_to_memory(mem_addr, values, memory)?;
    Ok(())
}

/// `StateRead::KeyRangeExtern` operation.
/// Uses a synchronous state read.
pub fn key_range_ext<S>(
    state_read: &S,
    stack: &mut Stack,
    memory: &mut Memory,
) -> OpResult<(), S::Error>
where
    S: StateRead,
{
    let mem_addr = pop_memory_address(stack)?;
    let values = read_key_range_ext(state_read, stack)?;
    write_values_to_memory(mem_addr, values, memory)?;
    Ok(())
}

/// Read the length and key from the top of the stack and read the associated words from state.
/// Uses a synchronous state read.
fn read_key_range<S>(
    state_read: &S,
    contract_addr: &ContentAddress,
    stack: &mut Stack,
) -> OpResult<Vec<Value>, S::Error>
where
    S: StateRead,
{
    let (key, num_keys) = pop_key_range_args(stack)?;
    state_read
        .key_range(contract_addr.clone(), key, num_keys)
        .map_err(OpError::StateRead)
}

/// Read the length, key and external contract address from the top of the stack and
/// read the associated words from state.
/// Uses a synchronous state read.
fn read_key_range_ext<S>(state_read: &S, stack: &mut Stack) -> OpResult<Vec<Value>, S::Error>
where
    S: StateRead,
{
    let (key, num_keys) = pop_key_range_args(stack)?;
    let contract_addr = ContentAddress(u8_32_from_word_4(stack.pop4()?));
    state_read
        .key_range(contract_addr, key, num_keys)
        .map_err(OpError::StateRead)
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
