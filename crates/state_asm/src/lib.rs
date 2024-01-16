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
    Halt,
    Jump,
    JumpIf,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Memory {
    Alloc,
    Free,
    Load,
    Store,
    Clear,
    ClearRange,
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
