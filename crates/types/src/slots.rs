//! # Slots
//! Data types that outline the inputs to an intent.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// The slots that this intent can read and a solver can fill.
pub struct Slots {
    /// The amount of decision variables the intent expects.
    pub decision_variables: u32,
    /// The slots that state is read into.
    pub state: Vec<StateSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// A slot for state values to be read into.
/// This is the result of running a single state read program.
pub struct StateSlot {
    /// The index of the state slot.
    pub index: u32,
    /// The amount of words to read into the slot.
    pub amount: u32,
    /// Which state read program to run.
    pub program_index: u16,
}

/// Helper function to calculate the length of the state.
/// Note that this is not the same as the length of the vector.
pub fn state_len(state: &[StateSlot]) -> Option<u32> {
    if state.is_empty() {
        return Some(0);
    }
    state.iter().try_fold(0, |acc, slot| {
        Some(acc.max(slot.index.checked_add(slot.amount)?))
    })
}
