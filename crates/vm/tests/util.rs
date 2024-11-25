// Cargo treats each test in `tests` as a crate, so for some tests some items
// are considered dead code.
#![allow(dead_code)]

use essential_vm::{
    types::{solution::SolutionData, ContentAddress, Key, PredicateAddress, Word},
    Access, StateRead,
};
use std::{
    collections::{BTreeMap, HashSet},
    future::{self, Ready},
};
use thiserror::Error;

pub const TEST_SET_CA: ContentAddress = ContentAddress([0xFF; 32]);
pub const TEST_PREDICATE_CA: ContentAddress = ContentAddress([0xAA; 32]);
pub const TEST_PREDICATE_ADDR: PredicateAddress = PredicateAddress {
    contract: TEST_SET_CA,
    predicate: TEST_PREDICATE_CA,
};
pub const TEST_SOLUTION_DATA: SolutionData = SolutionData {
    predicate_to_solve: TEST_PREDICATE_ADDR,
    decision_variables: vec![],
    state_mutations: vec![],
};

pub(crate) fn test_empty_keys() -> &'static HashSet<&'static [Word]> {
    static INSTANCE: std::sync::LazyLock<HashSet<&[Word]>> =
        std::sync::LazyLock::new(|| HashSet::with_capacity(0));
    &INSTANCE
}

pub(crate) fn test_solution_data_arr() -> &'static [SolutionData] {
    static INSTANCE: std::sync::LazyLock<[SolutionData; 1]> =
        std::sync::LazyLock::new(|| [TEST_SOLUTION_DATA]);
    &*INSTANCE
}

pub(crate) fn test_access() -> &'static Access<'static> {
    static INSTANCE: std::sync::LazyLock<Access> = std::sync::LazyLock::new(|| Access {
        data: test_solution_data_arr(),
        index: 0,
        mutable_keys: test_empty_keys(),
    });
    &INSTANCE
}

// A test `StateRead` implementation represented using a map.
#[derive(Clone)]
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

    /// Retrieve a key range.
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

        // Collect the words.
        let mut words = vec![];
        for _ in 0..num_words {
            let opt = self
                .get(&contract_addr)
                .ok_or(InvalidStateRead)?
                .get(&key)
                .cloned()
                .unwrap_or_default();
            words.push(opt);
            key = next_key(key).ok_or(InvalidStateRead)?;
        }
        Ok(words)
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
