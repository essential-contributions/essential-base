use std::collections::HashMap;
use std::collections::HashSet;

use anyhow::bail;
use anyhow::ensure;
use check::SolvedIntent;
use db::Address;
use db::Db;
use intent::Intent;
use solution::Solution;

pub mod check;
pub mod data;
pub mod db;
pub mod intent;
pub mod op;
pub mod solution;
pub mod state_read;

#[derive(Default)]
pub struct Server {
    db: Db,
    intent_pool: HashMap<Address, Intent>,
    deployed_intents: HashMap<Address, Intent>,
    accounts: KeyStore,
}

#[derive(Default)]
pub struct KeyStore {
    pub accounts: HashMap<u64, ed25519_dalek::SigningKey>,
    pub index: u64,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_individual(
        &mut self,
        intent: SolvedIntent,
        target_utility: u64,
    ) -> anyhow::Result<bool> {
        let solution = check::check(&mut self.db, &self.accounts, intent)?;
        if solution == target_utility {
            self.db.commit();
            Ok(true)
        } else {
            self.db.rollback();
            Ok(false)
        }
    }

    pub fn check(&mut self, solution: Solution) -> anyhow::Result<u64> {
        self.db.rollback();
        let mut utility = 0;
        let intents: HashSet<Address> = solution.transitions.iter().map(|t| t.intent).collect();
        for transition in solution.transitions {
            let Some(intent) = self
                .intent_pool
                .get(&transition.intent)
                .or_else(|| self.deployed_intents.get(&transition.intent))
            else {
                bail!("Intent not found");
            };
            if let Some(msg_input_intent) = &transition.input_message {
                ensure!(
                    intents.contains(&msg_input_intent.sender),
                    "Message input Intent not found"
                );
            }
            let solved_intent = SolvedIntent {
                intent: intent.clone(),
                solution: transition,
            };
            utility += check::check(&mut self.db, &self.accounts, solved_intent)?;
        }
        Ok(utility)
    }

    pub fn db(&mut self) -> &mut Db {
        &mut self.db
    }
}
