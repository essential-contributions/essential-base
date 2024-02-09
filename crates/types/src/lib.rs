use std::ops::Range;

pub mod conversions;
pub mod intent;
pub mod slots;
pub mod solution;

pub type ConstraintBytecode = Vec<u8>;
pub type StateReadBytecode = Vec<u8>;

pub type Hash = [u8; 32];
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentAddress(pub Hash);

pub type Word = u64;

pub type Key = [Word; 4];
pub type KeyRange = Range<Key>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersistentAddress {
    pub set: IntentAddress,
    pub intent: IntentAddress,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceAddress {
    Transient(IntentAddress),
    Persistent(PersistentAddress),
}

impl SourceAddress {
    pub fn persistent(set: IntentAddress, intent: IntentAddress) -> Self {
        SourceAddress::Persistent(PersistentAddress { set, intent })
    }
    pub fn transient(intent: IntentAddress) -> Self {
        SourceAddress::Transient(intent)
    }
    pub fn set_address(&self) -> &IntentAddress {
        match self {
            SourceAddress::Transient(intent) => intent,
            SourceAddress::Persistent(persistent) => &persistent.set,
        }
    }
    pub fn intent_address(&self) -> &IntentAddress {
        match self {
            SourceAddress::Transient(intent) => intent,
            SourceAddress::Persistent(persistent) => &persistent.intent,
        }
    }
}
