use std::collections::BTreeMap;
use std::iter::Peekable;

pub type Key = [u64; 4];

#[derive(Clone, Default)]
pub struct Db {
    data: BTreeMap<Key, u64>,
    staged: Option<BTreeMap<Key, u64>>,
}

struct KeyIter {
    key: Option<Key>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            staged: None,
        }
    }

    pub fn read_range(&self, key: &Key, amount: i32) -> Vec<Option<u64>> {
        match &self.staged {
            Some(staged) => {
                let iter = staged.range(*key..).map(|(k, v)| (*k, *v)).peekable();
                construct_values(*key, amount as u64, iter)
            }
            None => {
                let iter = self.data.range(*key..).map(|(k, v)| (*k, *v)).peekable();
                construct_values(*key, amount as u64, iter)
            }
        }
    }

    pub fn stage(&mut self, key: Key, value: Option<u64>) {
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

fn construct_values<I>(key: Key, amount: u64, mut iter: Peekable<I>) -> Vec<Option<u64>>
where
    I: Iterator<Item = (Key, u64)>,
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
    type Item = [u64; 4];

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.key;
        if let Some(key) = self.key {
            self.key = add_one(key, 0);
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
