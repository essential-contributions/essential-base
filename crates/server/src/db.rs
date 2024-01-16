use std::collections::BTreeMap;
use std::iter::Peekable;
use std::ops::Range;

pub type Key = [u64; 4];
pub type Address = [u64; 4];
pub type KeyRange = Range<Key>;

#[derive(Clone, Default)]
pub struct Db {
    data: BTreeMap<InnerKey, u64>,
    staged: Option<BTreeMap<InnerKey, u64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct InnerKey {
    address: Address,
    key: Key,
}

struct KeyIter {
    key: Option<InnerKey>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            staged: None,
        }
    }

    pub fn read_range(&self, address: &Address, key: &Key, amount: i32) -> Vec<Option<u64>> {
        let key = InnerKey {
            address: *address,
            key: *key,
        };
        match &self.staged {
            Some(staged) => {
                let iter = staged.range(key..).map(|(k, v)| (*k, *v)).peekable();
                construct_values(key, amount as u64, iter)
            }
            None => {
                let iter = self.data.range(key..).map(|(k, v)| (*k, *v)).peekable();
                construct_values(key, amount as u64, iter)
            }
        }
    }

    pub fn stage(&mut self, address: Address, key: Key, value: Option<u64>) {
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
}

pub fn key_range(key: Key, amount: u64) -> Option<KeyRange> {
    let mut end = key;
    for _ in 0..amount {
        end = add_one(end, 0)?;
    }
    Some(key..end)
}

fn construct_values<I>(key: InnerKey, amount: u64, mut iter: Peekable<I>) -> Vec<Option<u64>>
where
    I: Iterator<Item = (InnerKey, u64)>,
{
    KeyIter { key: Some(key) }
        .map(|k| (k, None::<u64>))
        .map(|(k, _)| match iter.peek() {
            Some((k2, _)) if k == *k2 => iter.next().map(|(_, v)| v),
            _ => None,
        })
        .take(amount as usize)
        .collect()
}

impl Iterator for KeyIter {
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

fn add_one(mut key: Key, index: usize) -> Option<Key> {
    if index >= key.len() {
        return None;
    }
    match key[index].checked_add(1) {
        Some(n) => {
            key[index] = n;
            Some(key)
        }
        None => add_one(key, index + 1),
    }
}
