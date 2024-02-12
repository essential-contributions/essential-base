pub use essential_types::slots::Slots;
use essential_types::{solution::Sender, SourceAddress};

#[derive(Clone, Debug)]
pub struct Data {
    pub source_address: SourceAddress,
    pub decision_variables: Vec<u64>,
    pub state: Vec<Option<u64>>,
    pub state_delta: Vec<Option<u64>>,
    pub sender: Sender,
}
