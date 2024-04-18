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
    type Error: std::error::Error;
    /// The future type returned from the `word_range` method.
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
    type Future<'s>: Future<Output = Result<Vec<Option<Word>>, Self::Error>> + Unpin + 's
    where
        Self: 's;

    /// Read the given number of words from state at the given key associated
    /// with the given intent set address.
    fn word_range(&self, set_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future<'_>;
}

/// A future representing the asynchronous `StateRead` (or `StateReadExtern`) operation.
///
/// Performs the state read and then writes the result to memory.
pub(crate) struct StateReadFuture<'vm, 's, S>
where
    S: StateRead + 's,
{
    /// The future produced by the `StateRead::word_range` implementation.
    future: S::Future<'s>,
    /// Access to the `Vm` so that the result of the future can be written to memory.
    pub(crate) vm: &'vm mut Vm,
}

impl<'vm, 's, S> Future for StateReadFuture<'vm, 's, S>
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
                let res = res
                    .map_err(OpAsyncError::StateRead)
                    .and_then(|words| write_words_to_memory(self.vm, words));
                Poll::Ready(res)
            }
        }
    }
}

/// `StateRead::WordRange` operation.
pub fn word_range<'vm, 's, S>(
    state_read: &'s S,
    set_addr: &ContentAddress,
    vm: &'vm mut Vm,
) -> OpAsyncResult<StateReadFuture<'vm, 's, S>, S::Error>
where
    S: StateRead,
{
    let future = read_word_range(state_read, set_addr, vm)?;
    Ok(StateReadFuture { future, vm })
}

/// `StateRead::WordRangeExtern` operation.
pub fn word_range_ext<'vm, 's, S>(
    state_read: &'s S,
    vm: &'vm mut Vm,
) -> OpAsyncResult<StateReadFuture<'vm, 's, S>, S::Error>
where
    S: StateRead,
{
    let future = read_word_range_ext(state_read, vm)?;
    Ok(StateReadFuture { future, vm })
}

/// Read the length and key from the top of the stack and read the associated words from state.
fn read_word_range<'s, S>(
    state_read: &'s S,
    set_addr: &ContentAddress,
    vm: &mut Vm,
) -> OpAsyncResult<S::Future<'s>, S::Error>
where
    S: StateRead,
{
    let len_word = vm.stack.pop()?;
    let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = vm.stack.pop4()?;
    Ok(state_read.word_range(set_addr.clone(), key, len))
}

/// Read the length, key and external set address from the top of the stack and
/// read the associated words from state.
fn read_word_range_ext<'s, S>(
    state_read: &'s S,
    vm: &mut Vm,
) -> OpAsyncResult<S::Future<'s>, S::Error>
where
    S: StateRead,
{
    let len_word = vm.stack.pop()?;
    let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = vm.stack.pop4().map_err(OpAsyncError::from)?;
    let set_addr = ContentAddress(u8_32_from_word_4(vm.stack.pop4()?));
    Ok(state_read.word_range(set_addr, key, len))
}

/// Write the given words to the end of memory and push the starting memory address to the stack.
fn write_words_to_memory<E>(vm: &mut Vm, words: Vec<Option<Word>>) -> OpAsyncResult<(), E> {
    let start = Word::try_from(vm.memory.len()).map_err(|_| StackError::IndexOutOfBounds)?;
    vm.memory.extend(words)?;
    vm.stack.push(start)?;
    Ok(())
}
