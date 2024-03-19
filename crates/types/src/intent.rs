//! # Intents
//! Types needed to represent an intent.

#[allow(unused_imports)]
use crate::{
    slots::{Slots, StateSlot},
    ConstraintBytecode, StateReadBytecode,
};
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use sha2::{Digest, Sha256};
#[allow(unused_imports)]
use std::ops::Deref;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_intent_serialization() {
        let intent = Intent {
            slots: Slots {
                decision_variables: 1,
                state: vec![
                    StateSlot {
                        index: 1,
                        amount: 2,
                        program_index: 3,
                    },
                    StateSlot {
                        index: 16,
                        amount: 17,
                        program_index: 18,
                    },
                ],
                permits: 1,
            },
            state_read: vec![vec![0x01, 0x02, 0x03], vec![0x10, 0x11]],
            constraints: vec![vec![0x20, 0x21, 0x22], vec![0x30, 0x31], vec![0x40]],
            directive: Directive::Maximize(vec![0x50, 0x51]),
        };
        let output: Vec<u8> = postcard::to_allocvec(&intent).unwrap();
        assert_eq!(
            &[
                0x01, 0x02, 0x01, 0x02, 0x03, 0x10, 0x11, 0x12, 0x01, 0x02, 0x03, 0x01, 0x02, 0x03,
                0x02, 0x10, 0x11, 0x03, 0x03, 0x20, 0x21, 0x22, 0x02, 0x30, 0x31, 0x01, 0x40, 0x01,
                0x02, 0x50, 0x51
            ],
            output.deref()
        );
        let out: Intent = postcard::from_bytes(output.deref()).unwrap();
        assert_eq!(out, intent);
    }

    #[test]
    pub fn test_intent_hashing() {
        let intent = Intent {
            slots: Slots {
                decision_variables: 1,
                state: vec![
                    StateSlot {
                        index: 1,
                        amount: 2,
                        program_index: 3,
                    },
                    StateSlot {
                        index: 16,
                        amount: 17,
                        program_index: 18,
                    },
                ],
                permits: 1,
            },
            state_read: vec![vec![0x01, 0x02, 0x03], vec![0x10, 0x11]],
            constraints: vec![vec![0x20, 0x21, 0x22], vec![0x30, 0x31], vec![0x40]],
            directive: Directive::Maximize(vec![0x50, 0x51]),
        };
        let mut hasher = Sha256::new();
        hasher.update(&postcard::to_allocvec(&intent).unwrap());
        let hash = hasher.finalize();

        assert_eq!(
            "aebc0cd28c9be9f6b6953286ce45b1cc1a015aa4f6692394990ec17f0ac89e57",
            format!("{:x}", hash)
        );
    }
}
