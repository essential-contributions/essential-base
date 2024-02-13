use std::collections::BTreeMap;
use std::iter::Peekable;
use std::ops::Range;

use essential_types::Word;

pub type Key = [Word; 4];
pub type Address = [Word; 4];
pub type PubKey = [Word; 4];
pub type KeyRange = Range<Key>;

#[derive(Clone, Default, Debug)]
pub struct Db {
    data: BTreeMap<InnerKey, Word>,
    staged: Option<BTreeMap<InnerKey, Word>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct InnerKey {
    address: Address,
    key: Key,
}

pub struct KeyRangeIter {
    key: Option<Key>,
    until: Key,
}

struct InnerKeyIter {
    key: Option<InnerKey>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            staged: None,
        }
    }

    pub fn read_range(&self, address: &Address, key: &Key, amount: i32) -> Vec<Option<Word>> {
        let key = InnerKey {
            address: *address,
            key: *key,
        };
        match &self.staged {
            Some(staged) => {
                let iter = staged.range(key..).map(|(k, v)| (*k, *v)).peekable();
                construct_values(key, amount as Word, iter)
            }
            None => {
                let iter = self.data.range(key..).map(|(k, v)| (*k, *v)).peekable();
                construct_values(key, amount as Word, iter)
            }
        }
    }

    pub fn stage(&mut self, address: Address, key: Key, value: Option<Word>) {
        let key = InnerKey { address, key };
        if let Some(staged) = &mut self.staged {
            match value {
                Some(value) => {
                    staged.insert(key, value);
                }
                None => {
                    staged.remove(&key);
                }
            }
        } else {
            let mut staged = self.data.clone();
            match value {
                Some(value) => {
                    staged.insert(key, value);
                }
                None => {
                    staged.remove(&key);
                }
            }
            self.staged = Some(staged);
        }
    }

    pub fn commit(&mut self) {
        if let Some(staged) = self.staged.take() {
            self.data = staged;
            self.staged = None;
        }
    }

    pub fn rollback(&mut self) {
        self.staged = None;
    }

    pub fn set_values(&self) -> impl Iterator<Item = (Address, Key, Word)> + '_ {
        self.data.iter().map(|(k, v)| (k.address, k.key, *v))
    }
}

pub fn key_range(key: Key, amount: Word) -> Option<KeyRange> {
    let mut end = key;
    for _ in 0..amount {
        end = add_one(end, 0)?;
    }
    Some(key..end)
}

fn construct_values<I>(key: InnerKey, amount: Word, mut iter: Peekable<I>) -> Vec<Option<Word>>
where
    I: Iterator<Item = (InnerKey, Word)>,
{
    InnerKeyIter { key: Some(key) }
        .map(|k| (k, None::<Word>))
        .map(|(k, _)| match iter.peek() {
            Some((k2, _)) if k == *k2 => iter.next().map(|(_, v)| v),
            _ => None,
        })
        .take(amount as usize)
        .collect()
}

impl Iterator for InnerKeyIter {
    type Item = InnerKey;

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.key;
        if let Some(key) = self.key {
            self.key = add_one(key.key, 0).map(|k| InnerKey {
                address: key.address,
                key: k,
            });
        }
        r
    }
}

impl KeyRangeIter {
    pub fn new(key: KeyRange) -> Self {
        Self {
            key: Some(key.start),
            until: key.end,
        }
    }
}

impl Iterator for KeyRangeIter {
    type Item = Key;

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.key;
        if let Some(key) = self.key {
            self.key = add_one(key, 0);
            if self.key == Some(self.until) {
                self.key = None;
            }
        }
        r
    }
}

fn add_one(key: Key, index: usize) -> Option<Key> {
    add_to_key(key, index, 1)
}

pub fn add_to_key(mut key: Key, index: usize, amount: Word) -> Option<Key> {
    if index >= key.len() {
        return None;
    }
    match key[index].checked_add(amount) {
        Some(n) => {
            key[index] = n;
            Some(key)
        }
        None => add_to_key(key, index + 1, amount),
    }
}
