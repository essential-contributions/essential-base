//! # Intents
//! Types needed to represent an intent.

use serde::{Deserialize, Serialize};

use crate::{slots::Slots, ConstraintBytecode, StateReadBytecode};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// A transient or deployed intent.
pub struct Intent {
    /// The slots that this intent can read.
    /// These are the inputs to the intent.
    /// They show up as read only registers available to both the
    /// state read and constraint programs.
    pub slots: Slots,
    /// The programs that read state.
    pub state_read: Vec<StateReadBytecode>,
    /// The programs that check constraints.
    pub constraints: Vec<ConstraintBytecode>,
    /// The directive for the intent.
    pub directive: Directive,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// The directive for the intent.
pub enum Directive {
    /// All constraints must be satisfied.
    Satisfy,
    /// Maximize the objective value.
    Maximize(ConstraintBytecode),
    /// Minimize the objective value.
    Minimize(ConstraintBytecode),
}
