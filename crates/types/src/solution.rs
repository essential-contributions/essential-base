use std::collections::BTreeMap;

use crate::{IntentAddress, Key, KeyRange, PersistentAddress, SourceAddress, Word};

#[derive(Debug, Clone)]
pub struct Solution {
    pub data: BTreeMap<SourceAddress, SolutionData>,
    pub state_mutations: Vec<StateMutation>,
}

#[derive(Debug, Clone)]
pub struct SolutionData {
    pub decision_variables: Vec<Word>,
    pub sender: Sender,
}

#[derive(Debug, Clone)]
pub enum Sender {
    Eoa([Word; 4]),
    Transient(TransientSender),
    Persistent(PersistentSender),
}

#[derive(Debug, Clone)]
pub struct TransientSender {
    pub eoa: [Word; 4],
    pub intent: IntentAddress,
}

#[derive(Debug, Clone)]
pub struct PersistentSender {
    pub set: IntentAddress,
    pub intent: IntentAddress,
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

impl Sender {
    pub fn eao(eoa: [Word; 4]) -> Self {
        Sender::Eoa(eoa)
    }
    pub fn transient(eoa: [Word; 4], intent: IntentAddress) -> Self {
        Sender::Transient(TransientSender { eoa, intent })
    }
    pub fn persistent(set: IntentAddress, intent: IntentAddress) -> Self {
        Sender::Persistent(PersistentSender { set, intent })
    }
    pub fn source_intent(&self) -> Option<SourceAddress> {
        match self {
            Sender::Eoa(_) => None,
            Sender::Transient(sender) => Some(SourceAddress::Transient(sender.intent.clone())),
            Sender::Persistent(sender) => Some(SourceAddress::Persistent(PersistentAddress {
                set: sender.set.clone(),
                intent: sender.intent.clone(),
            })),
        }
    }
}
