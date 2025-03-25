use essential_check::{
    sign::secp256k1::{PublicKey, Secp256k1, SecretKey},
    types::{solution::SolutionSet, ContentAddress, Key, PredicateAddress, Word},
    vm::StateRead,
};
use essential_types::{
    contract::{self, Contract},
    predicate::Predicate,
};
use std::collections::BTreeMap;
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

    /// Apply all mutations proposed by the given solution set.
    pub fn apply_mutations(&mut self, set: &SolutionSet) {
        for solution in &set.solutions {
            for mutation in solution.state_mutations.iter() {
                self.set(
                    solution.predicate_to_solve.contract.clone(),
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
    fn key_range(
        &self,
        contract_addr: ContentAddress,
        key: Key,
        num_words: usize,
    ) -> Result<Vec<Vec<Word>>, Self::Error> {
        self.key_range(contract_addr, key, num_words)
    }
}

pub fn empty_solution_set() -> SolutionSet {
    SolutionSet {
        solutions: Default::default(),
    }
}

pub fn empty_predicate() -> Predicate {
    Predicate::default()
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

pub fn contract_addr(predicates: &contract::SignedContract) -> ContentAddress {
    essential_hash::content_addr(&predicates.contract)
}

pub fn predicate_addr(predicates: &contract::SignedContract, ix: usize) -> PredicateAddress {
    PredicateAddress {
        contract: contract_addr(predicates),
        predicate: essential_hash::content_addr(&predicates.contract[ix]),
    }
}
