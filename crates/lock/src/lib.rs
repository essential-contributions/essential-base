//! # Lock
//! This crate is a simple wrapper around sync mutexes that makes it impossible to
//! hold a lock over an await.
//!
//! It also makes it a lot clearer where the bounds of the lock are which helps make deadlocks easier to debug
//! as a deadlock would require having the call to `apply` more than once on the stack.
#![deny(missing_docs)]
#![deny(unsafe_code)]

/// A lock that is guaranteed to be released before an await.
pub struct StdLock<T> {
    data: std::sync::Mutex<T>,
}

impl<T> StdLock<T> {
    /// Create a new lock with the given data.
    pub fn new(data: T) -> Self {
        StdLock {
            data: std::sync::Mutex::new(data),
        }
    }

    /// Apply a function to the data in the lock.
    pub fn apply<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        f(&mut self.data.lock().expect("Mutex was poisoned"))
    }
}
