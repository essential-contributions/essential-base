use std::collections::HashMap;

use anyhow::bail;
use anyhow::ensure;
use check::pack_bytes;
use check::SolvedIntent;
use db::Address;
use db::Db;
use db::PubKey;
use intent::intent_set_address;
use intent::Intent;
use intent::ToIntentAddress;
use solution::Solution;

use crate::check::unpack_bytes;

pub mod check;
pub mod data;
pub mod db;
pub mod intent;
pub mod solution;
pub mod state_read;

#[derive(Default)]
pub struct Server {
    db: Db,
    intent_pool: HashMap<Address, Intent>,
    deployed_intents: HashMap<Address, HashMap<Address, Intent>>,
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
        let solution = check::check(&mut self.db, intent)?;
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
        for (set, transition) in solution.data.clone() {
            let set: Address = set.into();
            let Some(intent) = self.intent_pool.get(&set).or_else(|| {
                transition
                    .input_message
                    .as_ref()
                    .and_then(|m| self.get_deployed(&set, &m.recipient.clone().into()))
            }) else {
                bail!("Intent not found");
            };
            if let Some(msg_input_intent) = &transition.input_message {
                let Some(message_intent) = solution.data.get(&msg_input_intent.sender) else {
                    bail!("Message input Intent not found");
                };
                let output_message_matches = message_intent
                    .output_messages
                    .iter()
                    .any(|o| o.args == msg_input_intent.args);
                ensure!(output_message_matches, "No matching output message found");
            }
            let solved_intent = SolvedIntent {
                intent: intent.clone(),
                deployed_address: set,
                solution: transition,
                state_mutations: solution.state_mutations.clone(),
            };
            utility += check::check(&mut self.db, solved_intent)?;
        }
        Ok(utility)
    }

    pub fn submit_intent(&mut self, intent: Intent) -> anyhow::Result<Address> {
        let address = intent.address();
        self.intent_pool.insert(address, intent);
        Ok(address)
    }

    pub fn deploy_intent_set(&mut self, intents: Vec<Intent>) -> anyhow::Result<Address> {
        let intents: HashMap<Address, Intent> =
            intents.into_iter().map(|i| (i.address(), i)).collect();
        let address = intent_set_address(intents.keys());
        self.deployed_intents.insert(address, intents);
        Ok(address)
    }

    pub fn submit_solution(&mut self, solution: Solution) -> anyhow::Result<u64> {
        let utility = self.check(solution)?;
        self.db.commit();
        Ok(utility)
    }

    pub fn list_intents(&self) -> impl Iterator<Item = (&Address, &Intent)> {
        self.intent_pool.iter()
    }

    pub fn list_deployed(
        &self,
    ) -> impl Iterator<Item = (&Address, impl Iterator<Item = (&Address, &Intent)>)> {
        self.deployed_intents.iter().map(|(k, v)| (k, v.iter()))
    }

    pub fn list_deployed_sets(&self) -> impl Iterator<Item = &Address> {
        self.deployed_intents.keys()
    }

    pub fn get_intent(&self, address: &Address) -> Option<&Intent> {
        self.intent_pool.get(address)
    }

    pub fn get_deployed(&self, set: &Address, address: &Address) -> Option<&Intent> {
        self.deployed_intents.get(set).and_then(|s| s.get(address))
    }

    pub fn get_deployed_set(&self, address: &Address) -> Option<&HashMap<Address, Intent>> {
        self.deployed_intents.get(address)
    }

    pub fn generate_account(&mut self) -> anyhow::Result<u64> {
        use rand_core::OsRng;

        let index = self.accounts.index;
        let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
        self.accounts.accounts.insert(index, signing_key);
        self.accounts.index += 1;
        Ok(index)
    }

    pub fn get_public_key(&self, index: u64) -> anyhow::Result<PubKey> {
        let signing_key = self
            .accounts
            .accounts
            .get(&index)
            .ok_or_else(|| anyhow::anyhow!("Account not found"))?;
        let key: Vec<_> = signing_key
            .verifying_key()
            .as_bytes()
            .chunks_exact(8)
            .map(pack_bytes)
            .collect();
        let Ok(key) = key.try_into() else {
            bail!("Invalid key length");
        };
        Ok(key)
    }

    pub fn db(&mut self) -> &mut Db {
        &mut self.db
    }
}

pub fn hash(bytes: &[u8]) -> Address {
    use sha2::Digest;

    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    let hash: [u8; 32] = hasher.finalize().into();
    let mut out = [0u64; 4];
    for (o, h) in out.iter_mut().zip(hash.chunks_exact(8)) {
        *o = pack_bytes(h);
    }
    out
}

pub fn hash_words(bytes: &[u64]) -> Address {
    use sha2::Digest;

    let bytes = bytes
        .iter()
        .copied()
        .flat_map(unpack_bytes)
        .collect::<Vec<_>>();

    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    let hash: [u8; 32] = hasher.finalize().into();
    let mut out = [0u64; 4];
    for (o, h) in out.iter_mut().zip(hash.chunks_exact(8)) {
        *o = pack_bytes(h);
    }
    out
}
