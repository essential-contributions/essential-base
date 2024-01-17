pub use constraint_asm;
pub use constraint_asm::Op;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum StateReadOp {
    Constraint(Op),
    State(State),
    ControlFlow(ControlFlow),
    Memory(Memory),
    Keys(Keys),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
pub enum Keys {
    /// Set available write keys to be overwritten.
    /// This means that the keys that are read will
    /// no longer be tracked.
    Overwrite,
    /// Push a range of keys into the key memory.
    /// This makes the key range available for writing.
    /// params -> {key: list len 4, amount}
    Push,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
