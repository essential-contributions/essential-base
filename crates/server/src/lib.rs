use check::Directive;
use check::SolvedIntent;
use data::Slots;
use db::Db;
use state_read::StateRead;

pub mod check;
pub mod data;
pub mod db;
pub mod op;
pub mod state_read;

pub struct Intent {
    pub slots: Slots,
    pub state_read: StateRead<Vec<u8>, Vec<u8>>,
    pub constraints: Vec<Vec<u8>>,
    pub directive: Directive,
}

#[derive(Default)]
pub struct Server {
    db: Db,
}

impl Server {
    pub fn new() -> Self {
        Self { db: Db::new() }
    }

    pub fn check(&mut self, intent: SolvedIntent, target_utility: u64) -> anyhow::Result<bool> {
        let solution = check::check(&mut self.db, intent)?;
        if solution == target_utility {
            self.db.commit();
            Ok(true)
        } else {
            self.db.rollback();
            Ok(false)
        }
    }

    pub fn db(&mut self) -> &mut Db {
        &mut self.db
    }
}
