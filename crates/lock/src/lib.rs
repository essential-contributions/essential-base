//! # Lock
//! This crate is a simple wrapper around sync mutexes that makes it impossible to
//! hold a lock over an await.
//!
//! It also makes it a lot clearer where the bounds of the lock are which helps make deadlocks easier to debug
//! as a deadlock would require having the call to `apply` more than once on the stack.
#![deny(missing_docs)]
#![deny(unsafe_code)]
use std::ops::DerefMut;

/// A lock that is guaranteed to be released before an await.
#[derive(Default)]
pub struct Lock<M> {
    mutex: M,
}

/// A generic trait for different types of mutexes.
pub trait Mutex {
    /// The data stored behind the `Mutex`.
    type Data: ?Sized;
    /// The `Guard` returned via [`Mutex::lock`].
    type Guard<'a>: DerefMut<Target = Self::Data>
    where
        Self: 'a;

    /// Create a new mutex with the given data.
    fn new(data: Self::Data) -> Self;

    /// Lock the mutex.
    fn lock(&self) -> Self::Guard<'_>;
}

impl<M> Lock<M>
where
    M: Mutex,
{
    /// Create a new lock with the given data.
    pub fn new(data: M::Data) -> Self
    where
        M::Data: Sized,
    {
        Lock {
            mutex: Mutex::new(data),
        }
    }

    /// Apply a function to the data in the lock.
    pub fn apply<U>(&self, f: impl FnOnce(&mut M::Data) -> U) -> U {
        f(&mut self.mutex.lock())
    }
}

impl<T> Mutex for std::sync::Mutex<T> {
    type Data = T;
    type Guard<'a> = std::sync::MutexGuard<'a, Self::Data> where T: 'a;

    fn new(data: T) -> Self {
        std::sync::Mutex::new(data)
    }

    fn lock(&self) -> Self::Guard<'_> {
        self.lock().unwrap()
    }
}
