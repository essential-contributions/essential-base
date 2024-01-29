use std::collections::BTreeMap;

use crate::{IntentAddress, Key, KeyRange, Word};

pub struct Solution {
    pub data: BTreeMap<IntentAddress, SolutionData>,
    pub state_mutations: Vec<StateMutation>,
}

pub struct SolutionData {
    pub decision_variables: Vec<Word>,
    pub input_message: Option<InputMessage>,
    pub output_messages: Vec<OutputMessage>,
}

pub struct InputMessage {
    pub sender: IntentAddress,
    pub recipient: IntentAddress,
    pub args: Vec<Vec<Word>>,
}

pub struct OutputMessage {
    pub args: Vec<Vec<Word>>,
}

pub struct KeyMutation {
    pub key: Key,
    pub value: Option<Word>,
}

pub struct RangeMutation {
    pub key_range: KeyRange,
    pub values: Vec<Option<Word>>,
}

pub enum Mutation {
    Key(KeyMutation),
    Range(RangeMutation),
}

pub struct StateMutation {
    pub address: IntentAddress,
    pub mutations: Vec<Mutation>,
}
