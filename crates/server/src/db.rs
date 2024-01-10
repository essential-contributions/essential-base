use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub struct Db {
    data: BTreeMap<u64, u64>,
    staged: Option<BTreeMap<u64, u64>>
}

impl Db {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            staged: None,
        }
    }

    pub fn read_range(&self, key: &u64, amount: i32) -> impl Iterator<Item = &u64> {
        self.data.range(key..).take(amount as usize).map(|(_, v)| v)
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
