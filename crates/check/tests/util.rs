use essential_check::{
    constraint_vm,
    sign::secp256k1::{PublicKey, Secp256k1, SecretKey},
    state_read_vm,
    state_read_vm::StateRead,
    types::{
        predicate::{Directive, Predicate},
        solution::{Mutation, Solution, SolutionData},
        ContentAddress, Key, PredicateAddress, Word,
    },
};
use essential_types::contract::{self, Contract};
use std::{
    collections::BTreeMap,
    future::{self, Ready},
};
use thiserror::Error;

// A test `StateRead` implementation represented using a map.
#[derive(Clone, Debug)]
pub struct State(BTreeMap<ContentAddress, BTreeMap<Key, Vec<Word>>>);

#[derive(Debug, Error)]
#[error("no value for the given contract, key pair")]
pub struct InvalidStateRead;

pub type Kv = (Key, Vec<Word>);

impl State {
    // Empry state, fine for tests unrelated to reading state.
    pub const EMPTY: Self = State(BTreeMap::new());

    // Shorthand test state constructor.
    pub fn new(contracts: Vec<(ContentAddress, Vec<Kv>)>) -> Self {
        State(
            contracts
                .into_iter()
                .map(|(addr, vec)| {
                    let map: BTreeMap<_, _> = vec.into_iter().collect();
                    (addr, map)
                })
                .collect(),
        )
    }

    // Update the value at the given key within the given contract address.
    pub fn set(&mut self, contract_addr: ContentAddress, key: &Key, value: Vec<Word>) {
        let contract = self.0.entry(contract_addr).or_default();
        if value.is_empty() {
            contract.remove(key);
        } else {
            contract.insert(key.clone(), value);
        }
    }

    pub fn deploy_namespace(&mut self, contract_addr: ContentAddress) {
        self.0.entry(contract_addr).or_default();
    }

    /// Retrieve a word range.
    pub fn key_range(
        &self,
        contract_addr: ContentAddress,
        mut key: Key,
        num_words: usize,
    ) -> Result<Vec<Vec<Word>>, InvalidStateRead> {
        // Get the key that follows this one.
        fn next_key(mut key: Key) -> Option<Key> {
            for w in key.iter_mut().rev() {
                match *w {
                    Word::MAX => *w = Word::MIN,
                    _ => {
                        *w += 1;
                        return Some(key);
                    }
                }
            }
            None
        }

        // If the predicate does not exist yet, assume `None`s as though predicate hasn't been deployed yet?
        let contract = match self.get(&contract_addr) {
            None => return Err(InvalidStateRead),
            Some(contract) => contract,
        };

        // Collect the words.
        let mut words = vec![];
        for _ in 0..num_words {
            let opt = contract.get(&key).cloned().unwrap_or_default();
            words.push(opt);
            key = next_key(key).ok_or(InvalidStateRead)?;
        }
        Ok(words)
    }

    /// Apply all mutations proposed by the given solution.
    pub fn apply_mutations(&mut self, solution: &Solution) {
        for data in &solution.data {
            for mutation in data.state_mutations.iter() {
                self.set(
                    data.predicate_to_solve.contract.clone(),
                    &mutation.key,
                    mutation.value.clone(),
                );
            }
        }
    }
}

impl core::ops::Deref for State {
    type Target = BTreeMap<ContentAddress, BTreeMap<Key, Vec<Word>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StateRead for State {
    type Error = InvalidStateRead;
    type Future = Ready<Result<Vec<Vec<Word>>, Self::Error>>;
    fn key_range(&self, contract_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        future::ready(self.key_range(contract_addr, key, num_words))
    }
}

pub fn empty_solution() -> Solution {
    Solution {
        data: Default::default(),
    }
}

pub fn empty_predicate() -> Predicate {
    Predicate {
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

pub fn empty_contract() -> Contract {
    Contract::without_salt(vec![empty_predicate()])
}

pub fn random_keypair(seed: [u8; 32]) -> (SecretKey, PublicKey) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::SmallRng::from_seed(seed);
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut rng)
}

// A simple predicate that expects the value of previously uncontract state slot with index 0 to be 42.
pub fn test_predicate_42(entropy: Word) -> Predicate {
    Predicate {
        // State read program to read state slot 0.
        state_read: vec![state_read_vm::asm::to_bytes([
            state_read_vm::asm::Stack::Push(1).into(),
            state_read_vm::asm::StateSlots::AllocSlots.into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(4).into(),
            state_read_vm::asm::Stack::Push(1).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::StateRead::KeyRange,
            state_read_vm::asm::TotalControlFlow::Halt.into(),
        ])
        .collect()],
        // Program to check pre-mutation value is None and
        // post-mutation value is 42 at slot 0.
        constraints: vec![constraint_vm::asm::to_bytes([
            state_read_vm::asm::Stack::Push(entropy).into(),
            state_read_vm::asm::Stack::Pop.into(),
            constraint_vm::asm::Stack::Push(0).into(), // slot
            constraint_vm::asm::Stack::Push(0).into(), // pre
            constraint_vm::asm::Access::StateLen.into(),
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Pred::Eq.into(),
            constraint_vm::asm::Stack::Push(0).into(), // slot
            constraint_vm::asm::Stack::Push(1).into(), // post
            constraint_vm::asm::Access::State.into(),
            constraint_vm::asm::Stack::Push(42).into(),
            constraint_vm::asm::Pred::Eq.into(),
            constraint_vm::asm::Pred::And.into(),
        ])
        .collect()],
        directive: Directive::Satisfy,
    }
}

pub fn contract_addr(predicates: &contract::SignedContract) -> ContentAddress {
    essential_hash::content_addr(&predicates.contract)
}

pub fn predicate_addr(predicates: &contract::SignedContract, ix: usize) -> PredicateAddress {
    PredicateAddress {
        contract: contract_addr(predicates),
        predicate: essential_hash::content_addr(&predicates.contract[ix]),
    }
}

// Creates a test `Predicate` along with a `Solution` that solves it.
pub fn test_predicate_42_solution_pair(
    entropy: Word,
    keypair_seed: [u8; 32],
) -> (contract::SignedContract, Solution) {
    // Create the test predicate, ensure its decision_variables match, and sign.
    let predicate = test_predicate_42(entropy);
    let (sk, _pk) = random_keypair(keypair_seed);
    let predicates = essential_sign::contract::sign(vec![predicate].into(), &sk);
    let predicate_addr = predicate_addr(&predicates, 0);

    // Construct the solution decision variables.
    // The first is an inline variable 42.
    let decision_variables = vec![vec![42]];

    // Create the solution.
    let solution = Solution {
        data: vec![SolutionData {
            predicate_to_solve: predicate_addr,
            decision_variables,
            state_mutations: vec![Mutation {
                key: vec![0, 0, 0, 0],
                value: vec![42],
            }],
            transient_data: vec![],
        }],
    };

    (predicates, solution)
}
