use std::collections::BTreeMap;
use std::iter::Peekable;

#[derive(Clone, Default)]
pub struct Db {
    data: BTreeMap<u64, u64>,
    staged: Option<BTreeMap<u64, u64>>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            staged: None,
        }
    }

    pub fn read_range(&self, key: &u64, amount: i32) -> Vec<Option<u64>> {
        match &self.staged {
            Some(staged) => {
                let iter = staged.range(key..).map(|(k, v)| (*k, *v)).peekable();
                construct_values(*key, amount as u64, iter)
            }
            None => {
                let iter = self.data.range(key..).map(|(k, v)| (*k, *v)).peekable();
                construct_values(*key, amount as u64, iter)
            }
        }
    }

    pub fn stage(&mut self, key: u64, value: Option<u64>) {
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

fn construct_values<I>(key: u64, amount: u64, mut iter: Peekable<I>) -> Vec<Option<u64>>
where
    I: Iterator<Item = (u64, u64)>,
{
    (key..(key + amount))
        .map(|k| (k, None::<u64>))
        .map(|(k, _)| match iter.peek() {
            Some((k2, _)) if k == *k2 => iter.next().map(|(_, v)| v),
            _ => None,
        })
        .take(amount as usize)
        .collect()
}
