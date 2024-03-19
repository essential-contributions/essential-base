//! # Assembly for state read operations.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use essential_constraint_asm::{self as constraint_asm, Op};
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Set of operations that can be performed on the state.
pub enum StateReadOp {
    /// All the operations available to the constraint checker.
    Constraint(Op),
    /// Operations for reading state.
    State(State),
    /// Operations for controlling the flow of the program.
    ControlFlow(ControlFlow),
    /// Operations for controlling the memory.
    Memory(Memory),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Control flow operations.
pub enum ControlFlow {
    /// End the execution of the program and return the keys and memory.
    Halt,
    /// Jump to the given address.
    /// params -> address
    Jump,
    /// Jump to the given address if the value is true.
    /// params -> {address, value: bool}
    JumpIf,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Memory operations.
pub enum Memory {
    /// Allocate new memory to the end of the memory.
    /// params -> size
    Alloc,
    /// Free the given size of memory from the end of the memory.
    /// params -> size
    Free,
    /// Truncate the memory to the given size.
    /// This does not effect the capacity of the memory.
    /// params -> size
    Truncate,
    /// Load the index of memory onto the stack.
    /// params -> index
    /// return -> value
    ///
    /// Panics if the index is out of bounds.
    /// Returns 0 if the value is None.
    /// Use `IsSome` to check if the value is None.
    Load,
    /// Store the value at the index of memory.
    /// params -> {index, value}
    ///
    /// Panics if the index is out of bounds.
    Store,
    /// Push the value onto the end of the memory.
    /// params -> value
    ///
    /// Panics if not enough memory is allocated.
    Push,
    /// Similar to push except that it pushes a None value.
    ///
    /// Panics if not enough memory is allocated.
    PushNone,
    /// Set the value at the index of memory to None.
    /// params -> index
    ///
    /// Panics if the index is out of bounds.
    Clear,
    /// Set a range of memory to None.
    /// params -> {index, amount}
    ///
    /// Panics if the index..(index + amount) is out of bounds.
    ClearRange,
    /// Check if the value at the index of memory is Some.
    /// params -> index
    ///
    /// Panics if the index is out of bounds.
    IsSome,
    /// Get the current capacity of the memory.
    /// return -> capacity
    Capacity,
    /// Get the current length of the memory.
    /// return -> length
    Length,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Operations for reading state.
pub enum State {
    /// Read a range of words from state starting at the key.
    /// params -> {key: list len 4, amount}
    /// return -> address in memory where the words are stored
    ///
    /// Panics if not enough memory is allocated.
    StateReadWordRange,
    /// Read a range of words from external state starting at the key.
    /// params -> {extern_address: list len 4, key: list len 4, amount}
    /// return -> address in memory where the words are stored
    ///
    /// Panics if not enough memory is allocated.
    StateReadWordRangeExtern,
}
