use serde::{Deserialize, Serialize};

use crate::{slots::Slots, ConstraintBytecode, StateReadBytecode};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Intent {
    pub slots: Slots,
    pub state_read: Vec<StateReadBytecode>,
    pub constraints: Vec<ConstraintBytecode>,
    pub directive: Directive,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Directive {
    Satisfy,
    Maximize(ConstraintBytecode),
    Minimize(ConstraintBytecode),
}
