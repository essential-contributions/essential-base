// Cargo treats each test in `tests` as a crate, so for some tests some items
// are considered dead code.
#![allow(dead_code)]

use essential_state_read_vm::{
    types::{solution::SolutionData, ContentAddress, IntentAddress, Key, Word},
    Access, SolutionAccess, StateRead, StateSlots,
};
use std::{
    collections::{BTreeMap, HashSet},
    future::{self, Ready},
};
use thiserror::Error;

pub const TEST_SET_CA: ContentAddress = ContentAddress([0xFF; 32]);
pub const TEST_INTENT_CA: ContentAddress = ContentAddress([0xAA; 32]);
pub const TEST_INTENT_ADDR: IntentAddress = IntentAddress {
    set: TEST_SET_CA,
    intent: TEST_INTENT_CA,
};
pub const TEST_SOLUTION_DATA: SolutionData = SolutionData {
    intent_to_solve: TEST_INTENT_ADDR,
    decision_variables: vec![],
};

pub(crate) fn test_empty_keys() -> &'static HashSet<&'static [Word]> {
    static INSTANCE: once_cell::sync::OnceCell<HashSet<&[Word]>> = once_cell::sync::OnceCell::new();
    INSTANCE.get_or_init(|| HashSet::with_capacity(0))
}

pub(crate) fn test_solution_data_arr() -> &'static [SolutionData] {
    static INSTANCE: once_cell::sync::OnceCell<[SolutionData; 1]> =
        once_cell::sync::OnceCell::new();
    INSTANCE.get_or_init(|| [TEST_SOLUTION_DATA])
}

pub(crate) fn test_solution_access() -> &'static SolutionAccess<'static> {
    static INSTANCE: once_cell::sync::OnceCell<SolutionAccess> = once_cell::sync::OnceCell::new();
    INSTANCE.get_or_init(|| SolutionAccess {
        data: test_solution_data_arr(),
        index: 0,
        mutable_keys: test_empty_keys(),
    })
}

pub(crate) fn test_access() -> &'static Access<'static> {
    static INSTANCE: once_cell::sync::OnceCell<Access> = once_cell::sync::OnceCell::new();
    INSTANCE.get_or_init(|| Access {
        solution: *test_solution_access(),
        state_slots: StateSlots::EMPTY,
    })
}

// A test `StateRead` implementation represented using a map.
#[derive(Clone)]
pub struct State(BTreeMap<ContentAddress, BTreeMap<Key, Word>>);

#[derive(Debug, Error)]
#[error("no value for the given intent set, key pair")]
pub struct InvalidStateRead;

impl State {
    // Empry state, fine for tests unrelated to reading state.
    pub const EMPTY: Self = State(BTreeMap::new());

    // Shorthand test state constructor.
    pub fn new(sets: Vec<(ContentAddress, Vec<(Key, Word)>)>) -> Self {
        State(
            sets.into_iter()
                .map(|(addr, vec)| {
                    let map: BTreeMap<_, _> = vec.into_iter().collect();
                    (addr, map)
                })
                .collect(),
        )
    }

    // Update the value at the given key within the given intent set address.
    pub fn set(&mut self, set_addr: ContentAddress, key: &Key, value: Option<Word>) {
        let set = self.0.entry(set_addr).or_default();
        match value {
            None => {
                set.remove(key);
            }
            Some(value) => {
                set.insert(*key, value);
            }
        }
    }

    /// Retrieve a word range.
    pub fn word_range(
        &self,
        set_addr: ContentAddress,
        mut key: Key,
        num_words: usize,
    ) -> Result<Vec<Option<Word>>, InvalidStateRead> {
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
                .get(&set_addr)
                .ok_or(InvalidStateRead)?
                .get(&key)
                .cloned();
            words.push(opt);
            key = next_key(key).ok_or(InvalidStateRead)?;
        }
        Ok(words)
    }
}

impl core::ops::Deref for State {
    type Target = BTreeMap<ContentAddress, BTreeMap<Key, Word>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StateRead for State {
    type Error = InvalidStateRead;
    type Future = Ready<Result<Vec<Option<Word>>, Self::Error>>;
    fn word_range(&self, set_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        future::ready(self.word_range(set_addr, key, num_words))
    }
}
