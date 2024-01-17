use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Op {
    /// Push word onto stack.
    Push(u64),
    /// Pop word from stack.
    Pop,
    /// Duplicate top word on stack.
    Dup,
    /// Swap top two words on stack.
    Swap,
    Pred(Pred),
    Alu(Alu),
    Access(Access),
    Crypto(Crypto),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
    /// params -> word_index
    /// return -> word
    ///
    /// Word index must be in range or vm will panic.
    /// There must be an input message or vm will panic.
    InputMsgSenderWord,
    /// params -> ()
    /// return -> words: list with len 4
    ///
    /// There must be an input message or vm will panic.
    InputMsgSender,
    /// params -> {arg_index, word_index}
    /// return -> arg_word
    ///
    /// Arg and word index must be in range or vm will panic.
    /// There must be an input message or vm will panic.
    InputMsgArgWord,
    /// params -> {arg_index, start, end}
    /// return -> arg_words: list with len (end - start)
    ///
    /// Arg and start..end must be in range or vm will panic.
    /// There must be an input message or vm will panic.
    InputMsgArgRange,
    /// params -> arg_index
    /// return -> {arg_words: list, arg_len}
    ///
    /// Arg index must be in range or vm will panic.
    /// There must be an input message or vm will panic.
    InputMsgArg,
    /// params -> {msg_index, arg_index, word_index}
    /// return -> arg_word
    ///
    /// Msg, arg and word index must be in range or vm will panic.
    OutputMsgArgWord,
    /// params -> {msg_index, arg_index, start, end}
    /// return -> arg_words: list with len (end - start)
    ///
    /// Msg, arg and start..end must be in range or vm will panic.
    OutputMsgArgRange,
    /// params -> {msg_index, arg_index}
    /// return -> {arg_words: list, arg_len}
    ///
    /// Msg and arg index must be in range or vm will panic.
    OutputMsgArg,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
pub enum Crypto {
    /// params -> {data, data_len}
    /// return -> hash: list len 4
    Sha256,
    /// params -> {data, data_len, account_index}
    /// return -> signature: list len 8
    SignEd25519,
    /// params -> {data, data_len, signature: list len 8, account_index}
    /// return -> bool
    VerifyEd25519,
}
