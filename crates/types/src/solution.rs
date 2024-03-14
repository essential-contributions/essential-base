//! # Solutions
//! Data types that are used to create solutions to intents.

use serde::{Deserialize, Serialize};

use crate::{IntentAddress, Key, KeyRange, PublicKey, Word};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A solution to intents.
pub struct Solution {
    /// The input data for each intent.
    pub data: Vec<SolutionData>,
    /// The state mutations being proposed.
    pub state_mutations: Vec<StateMutation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The data the solver is required to provide to solve an intent.
pub struct SolutionData {
    /// Which intent this input data is for.
    pub intent_to_solve: IntentAddress,
    /// The decision variables for the intent.
    pub decision_variables: Vec<Word>,
    /// The EOA or intent that is permitting this intent to be solved.
    ///
    /// Not required in the case that an intent has no constraints over `context::owner()`.
    pub owner: Option<Owner>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The owner of permission to solve an intent.
pub enum Owner {
    /// This intent is being solved on behalf of an EOA.
    Eoa(PublicKey),
    /// This intent is being solved on behalf of an EOA forwarded via an intent.
    ForwardedEoa(ForwardedEoa),
    /// This intent is being solved on behalf of another intent.
    Intent(IntentAddress),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The owner is an EOA being forwarded via an intent.
pub struct ForwardedEoa {
    /// The EOA being forwarded.
    pub public_key: PublicKey,
    /// The intent from which the EOA was forwarded.
    pub intent: IntentAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A mutation to a single key in state.
pub struct KeyMutation {
    /// Key of state.
    pub key: Key,
    /// Value to set the key to.
    /// None means the value is being deleted.
    pub value: Option<Word>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Mutations to a range of keys in state.
/// This is more space efficient than a list of key mutations.
pub struct RangeMutation {
    /// The range of consecutive keys to mutate.
    pub key_range: KeyRange,
    /// The values to set the keys to.
    /// Must be the same length as the range.
    pub values: Vec<Option<Word>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The type of mutation to state.
pub enum Mutation {
    /// Mutation to a single key in state.
    Key(KeyMutation),
    /// Mutations to a range of keys in state.
    Range(RangeMutation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The state that is being proposed to be mutated.
/// This state is owned by the persistent intent.
pub struct StateMutation {
    /// The content address of the persistent intent.
    pub address: IntentAddress,
    /// The mutations to the state.
    pub mutations: Vec<Mutation>,
}

impl Owner {
    /// Get the source intent of the owner if it is not from an EOA.
    pub fn source_intent(&self) -> Option<&IntentAddress> {
        match self {
            Self::Eoa(_) => None,
            Self::ForwardedEoa(forwarded) => Some(&forwarded.intent),
            Self::Intent(intent) => Some(intent),
        }
    }
}
