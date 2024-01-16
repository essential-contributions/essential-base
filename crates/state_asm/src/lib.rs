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
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Keys {
    Overwrite,
    Push,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum State {
    StateReadWordRange,
    StateReadWordRangeExtern,
}
