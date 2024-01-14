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
