use std::collections::BTreeMap;

use crate::{IntentAddress, Key, KeyRange, Word};

#[derive(Debug, Clone)]
pub struct Solution {
    pub data: BTreeMap<IntentAddress, SolutionData>,
    pub state_mutations: Vec<StateMutation>,
}

#[derive(Debug, Clone, Default)]
pub struct SolutionData {
    pub decision_variables: Vec<Word>,
    pub input_message: Option<InputMessage>,
    pub output_messages: Vec<OutputMessage>,
}

#[derive(Debug, Clone)]
pub struct InputMessage {
    pub sender: IntentAddress,
    pub recipient: IntentAddress,
    pub args: Vec<Vec<Word>>,
}

#[derive(Debug, Clone)]
pub struct OutputMessage {
    pub args: Vec<Vec<Word>>,
}

#[derive(Debug, Clone)]
pub struct KeyMutation {
    pub key: Key,
    pub value: Option<Word>,
}

#[derive(Debug, Clone)]
pub struct RangeMutation {
    pub key_range: KeyRange,
    pub values: Vec<Option<Word>>,
}

#[derive(Debug, Clone)]
pub enum Mutation {
    Key(KeyMutation),
    Range(RangeMutation),
}

#[derive(Debug, Clone)]
pub struct StateMutation {
    pub address: IntentAddress,
    pub mutations: Vec<Mutation>,
}
