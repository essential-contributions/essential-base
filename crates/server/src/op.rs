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
    DecisionVar,
    DecisionVarRange,
    State,
    StateRange,
    StateIsSome,
    StateIsSomeRange,
    InputMsgSenderWord,
    InputMsgSender,
    InputMsgArgWord,
    InputMsgArg,
    InputMsgArgRange,
    OutputMsgArgWord,
    OutputMsgArg,
    OutputMsgArgRange,
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
