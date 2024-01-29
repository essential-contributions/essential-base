use std::ops::Range;

pub mod intent;
pub mod slots;
pub mod solution;

pub type ConstraintBytecode = Vec<u8>;
pub type StateReadBytecode = Vec<u8>;

pub type Hash = [u8; 32];
pub struct IntentAddress(pub Hash);

pub type Word = u64;

pub type Key = [Word; 4];
pub type KeyRange = Range<Key>;
