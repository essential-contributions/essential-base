pub use essential_types::slots::Slots;
use essential_types::{solution::Sender, SourceAddress, Word};

#[derive(Clone, Debug)]
pub struct Data {
    pub source_address: SourceAddress,
    pub decision_variables: Vec<Word>,
    pub state: Vec<Option<Word>>,
    pub state_delta: Vec<Option<Word>>,
    pub sender: Sender,
}
