#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! # Assembly for checking constraints.

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Set of operations that can be performed to check constraints.
pub enum Op {
    /// Push word onto stack.
    Push(i64),
    /// Pop word from stack.
    Pop,
    /// Duplicate top word on stack.
    Dup,
    /// Params -> index.
    /// Duplicate word at index counting from the top of the stack and push it onto the stack.
    DupFrom,
    /// Swap top two words on stack.
    Swap,
    /// Operations for computing predicates.
    Pred(Pred),
    /// Operations for computing arithmetic and logic.
    Alu(Alu),
    /// Operations for accessing input data.
    Access(Access),
    /// Operations for performing cryptographic operations.
    Crypto(Crypto),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Operations for accessing input data.
pub enum Access {
    /// params -> slot
    /// return -> decision_word
    ///
    /// Slot must be in range or vm will panic.
    DecisionVar,
    /// params -> {slot, range}
    /// return -> decision_words: list len range
    ///
    /// Slot..(Slot + Range) must be in range or vm will panic.
    DecisionVarRange,
    /// params -> {slot, delta: bool}
    /// return -> state_word
    ///
    /// Slot must be in range or vm will panic.
    /// Empty slots will be returned as 0.
    /// Use StateIsSome to check if a slot is empty.
    State,
    /// params -> {slot, range, delta: bool}
    /// return -> state_words: list len range
    ///
    /// Slot..(Slot + Range) must be in range or vm will panic.
    /// Empty slots will be returned as 0.
    /// Use StateIsSome to check if a slot is empty.
    StateRange,
    /// params -> {slot, delta: bool}
    /// return -> is_some: bool
    ///
    /// Slot must be in range or vm will panic.
    StateIsSome,
    /// params -> {slot, range, delta: bool}
    /// return -> is_somes: list of bools with len range
    ///
    /// Slot..(Slot + Range) must be in range or vm will panic.
    StateIsSomeRange,
    /// return -> words: list with len 4
    /// Returns the sender that permitted this intent.
    Sender,
    /// Params -> slot,
    /// Return -> key: list len 4
    ///
    /// Returns the key that is being proposed for mutation at this slot.
    MutKey,
    /// Return -> word
    ///
    /// Get the number of mutable keys being proposed for mutation.
    MutKeyLen,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Operations for computing predicates.
pub enum Pred {
    /// params -> {lhs, rhs}
    /// return -> bool
    Eq,
    /// params -> {lhs, rhs}
    /// return -> bool
    Gt,
    /// params -> {lhs, rhs}
    /// return -> bool
    Lt,
    /// params -> {lhs, rhs}
    /// return -> bool
    Gte,
    /// params -> {lhs, rhs}
    /// return -> bool
    Lte,
    /// params -> {lhs, rhs}
    /// return -> bool
    And,
    /// params -> {lhs, rhs}
    /// return -> bool
    Or,
    /// params -> word
    /// return -> bool
    Not,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Operations for computing arithmetic and logic.
pub enum Alu {
    /// params -> {lhs, rhs}
    /// return -> word
    Add,
    /// params -> {lhs, rhs}
    /// return -> word
    Sub,
    /// params -> {lhs, rhs}
    /// return -> word
    Mul,
    /// params -> {lhs, rhs}
    /// return -> word
    Div,
    /// params -> {lhs, rhs}
    /// return -> word
    Mod,
    /// Adds the offset to the hash.
    /// params -> {hash: list len 4, offset}
    /// return -> new_hash: list len 4
    HashOffset,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Operations for performing cryptographic operations.
pub enum Crypto {
    /// params -> {data, data_len}
    /// return -> hash: list len 4
    Sha256,
    /// params -> {data, data_len, signature: list len 8, public_key: list len 4}
    /// return -> bool
    VerifyEd25519,
}
