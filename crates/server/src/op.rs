use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Op {
    Push(u64),
    Pop,
    Dup,
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
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
    And,
    Or,
    Not,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Alu {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Crypto {
    /// [{data to hash}, data_len]
    /// [{hash: [u64; 4]}]
    Sha256,
    /// [{data to sign}, data_len, account_index]
    /// [{signature: [u64; 8]}]
    SignEd25519,
    /// [{data to verify}, data_len, {signature: [u64; 8]}, account_index]
    /// [verified]
    VerifyEd25519,
}
