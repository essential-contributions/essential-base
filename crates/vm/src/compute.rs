//! Compute operation implementations.

use crate::{
    error::{ComputeError, OpAsyncError, OpAsyncResult},
    StateRead, Vm,
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use essential_types::Word;
use tokio::task::{JoinError, JoinSet};

/// A future representing the asynchronous `Compute` operation.
///
/// Runs the compute threads and then conciles the resulting local memories.
pub(crate) struct ComputeFuture<S> {
    _state_read: std::marker::PhantomData<S>,
    set: JoinSet<Result<Vec<Word>, ComputeError>>,
    pub(crate) vm: Vm,
}

impl<S> Future for ComputeFuture<S>
where
    S: StateRead,
{
    /// Returns a `Result` representing whether or not the compute successful and the memory
    /// was conciled successfully.
    type Output = OpAsyncResult<(), S::Error>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut set_future = std::pin::pin!(self.set.join_next());
        match Pin::new(&mut set_future).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => match result {
                Some(res) => {
                    let res = res
                        .map_err(ComputeError::Join)
                        .map_err(OpAsyncError::Compute)
                        .and_then(|set_res| {
                            set_res
                                .map_err(OpAsyncError::Compute)
                                .and_then(|words| reconcile_memory(words, self.vm))
                        });
                    Poll::Ready(res)
                }
                None => Poll::Pending,
            },
        }
    }
}

/// `Compute::Compute` operation.
pub fn compute<S>() -> OpAsyncResult<ComputeFuture<S>, S::Error>
where
    S: StateRead,
{
    todo!()
}

fn reconcile_memory<E>(_values: Vec<Word>, _vm: Vm) -> OpAsyncResult<(), E> {
    todo!()
}
