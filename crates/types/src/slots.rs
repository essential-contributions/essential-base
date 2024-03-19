//! # Slots
//! Data types that outline the inputs to an intent.
use std::ops::Deref;
use serde::{Deserialize, Serialize};
extern crate alloc;
use alloc::vec::Vec;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// The slots that this intent can read and a solver can fill.
pub struct Slots {
    /// The amount of decision variables the intent expects.
    pub decision_variables: u32,
    /// The slots that state is read into.
    pub state: Vec<StateSlot>,
    /// The amount of other persistent intents that are permitted to be solved
    /// using this intent as the sender.
    pub permits: u16,
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

#[test]
pub fn test_state_slot_postcard() {
    let slot = StateSlot {
        index: 1,
        amount: 17,
        program_index: 255,
    };
    let output: Vec<u8> = postcard::to_allocvec(&slot).unwrap();
    assert_eq!(&[0x01, 0x11, 0xff, 0x01], output.deref());
    let out: StateSlot = postcard::from_bytes(output.deref()).unwrap();
    assert_eq!(out, slot);
}

#[test]
pub fn test_slots_postcard() {
    let slots = Slots {
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
        permits: 255,
    };
    let output: Vec<u8> = postcard::to_allocvec(&slots).unwrap();
    assert_eq!(&[0x01, 0x02, 0x01, 0x02, 0x03, 0x10, 0x11, 0x12, 0xff, 0x01], output.deref());
    let out: Slots = postcard::from_bytes(output.deref()).unwrap();
    assert_eq!(out, slots);
}
