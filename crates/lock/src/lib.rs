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
pub struct Lock<T, M = std::sync::Mutex<T>>
where
    M: Mutex<T>,
{
    data: M,
    phantom: std::marker::PhantomData<T>,
}

/// A generic trait for different types of mutexes.
pub trait Mutex<T: ?Sized> {
    /// Create a new mutex with the given data.
    fn new(data: T) -> Self;

    /// Lock the mutex.
    fn lock(&self) -> impl DerefMut<Target = T>;
}

impl<T, M> Lock<T, M>
where
    M: Mutex<T>,
{
    /// Create a new lock with the given data.
    pub fn new(data: T) -> Self {
        Lock {
            data: Mutex::new(data),
            phantom: std::marker::PhantomData,
        }
    }

    /// Apply a function to the data in the lock.
    pub fn apply<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        f(&mut self.data.lock())
    }
}

impl<T> Mutex<T> for std::sync::Mutex<T> {
    fn new(data: T) -> Self {
        std::sync::Mutex::new(data)
    }

    fn lock(&self) -> impl DerefMut<Target = T> {
        self.lock().unwrap()
    }
}
