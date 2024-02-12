//! # Solutions
//! Data types that are used to create solutions to intents.

use serde::{Deserialize, Serialize};

use crate::{Eoa, IntentAddress, Key, KeyRange, PersistentAddress, SourceAddress, Word};

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
    pub intent_to_solve: SourceAddress,
    /// The decision variables for the intent.
    pub decision_variables: Vec<Word>,
    /// The EOA or intent that is permitting this intent to be solved.
    pub sender: Sender,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The sender of permission to solve an intent.
pub enum Sender {
    /// This intent is being solved on behalf of an EOA.
    Eoa(Eoa),
    /// This intent is being solved on behalf of a transient intent.
    Transient(TransientSender),
    /// This intent is being solved on behalf of a persistent intent.
    Persistent(PersistentSender),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The data from a transient sender.
pub struct TransientSender {
    /// The EOA that submitted the transient intent.
    pub eoa: [Word; 4],
    /// The content address of the transient intent.
    pub intent: IntentAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The data from a persistent sender.
pub struct PersistentSender {
    /// The content address of the set of intents that this intent is deployed with.
    pub set: IntentAddress,
    /// The content address of the intent.
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

impl Sender {
    /// Construct a sender for an EOA.
    pub fn eao(eoa: Eoa) -> Self {
        Sender::Eoa(eoa)
    }

    /// Construct a sender for a transient intent.
    pub fn transient(eoa: Eoa, intent: IntentAddress) -> Self {
        Sender::Transient(TransientSender { eoa, intent })
    }

    /// Construct a sender for a persistent intent.
    pub fn persistent(set: IntentAddress, intent: IntentAddress) -> Self {
        Sender::Persistent(PersistentSender { set, intent })
    }

    /// Get the source intent of the sender if it is not from an EOA.
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
