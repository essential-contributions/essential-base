use crate::check::Transition;
use crate::db::Address;
use crate::db::Key;
use crate::db::KeyRange;

pub struct Solution {
    pub transitions: Vec<Transition>,
    pub state_mutations: StateMutations,
}

#[derive(Debug, Clone)]
pub struct KeyMutation {
    pub key: Key,
    pub value: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RangeMutation {
    pub key_range: KeyRange,
    pub values: Vec<Option<u64>>,
}

#[derive(Debug, Clone)]
pub enum Mutation {
    Key(KeyMutation),
    Range(RangeMutation),
}

#[derive(Debug, Clone)]
pub struct StateMutation {
    pub address: Address,
    pub mutations: Vec<Mutation>,
}

#[derive(Debug, Clone, Default)]
pub struct StateMutations {
    pub mutations: Vec<StateMutation>,
}
