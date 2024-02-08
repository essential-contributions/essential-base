use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Slots {
    pub decision_variables: u32,
    pub state: Vec<StateSlot>,
    pub input_message_args: Option<Vec<u16>>,
    pub output_messages: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StateSlot {
    pub index: u32,
    pub amount: u32,
    pub program_index: u16,
}

pub fn state_len(state: &[StateSlot]) -> Option<u32> {
    if state.is_empty() {
        return Some(0);
    }
    state.iter().try_fold(0, |acc, slot| {
        Some(acc.max(slot.index.checked_add(slot.amount)?))
    })
}
