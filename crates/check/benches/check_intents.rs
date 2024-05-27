use std::{
    collections::BTreeMap,
    future::{self, Ready},
    sync::Arc,
};

use criterion::{criterion_group, criterion_main, Criterion};
use essential_constraint_vm as constraint_vm;
use essential_state_read_vm as state_read_vm;
use essential_state_read_vm::StateRead;
use essential_types::{
    intent::{Directive, Intent},
    solution::{Mutation, Mutations, Solution, SolutionData},
    ContentAddress, IntentAddress, Key, Signed, Word,
};
use secp256k1::{PublicKey, Secp256k1, SecretKey};

pub fn bench(c: &mut Criterion) {
    let (intents, solution) = test_intent_42_solution_pair(1, [0; 32]);
    let intent = Arc::new(intents.data[0].clone());
    let get_intent = |_addr: &IntentAddress| intent.clone();
    let solution = Arc::new(solution);
    let config = Arc::new(essential_check::solution::CheckIntentConfig::default());
    let mut pre_state = State::EMPTY;
    pre_state.deploy_namespace(essential_hash::intent_set_addr::from_intents(&intents.data));
    let mut post_state = pre_state.clone();
    post_state.apply_mutations(&solution);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    c.bench_function("check_42", |b| {
        b.to_async(&runtime).iter(|| {
            essential_check::solution::check_intents(
                &pre_state,
                &post_state,
                solution.clone(),
                get_intent,
                config.clone(),
            )
        });
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);

#[derive(Clone, Debug)]
pub struct State(BTreeMap<ContentAddress, BTreeMap<Key, Vec<Word>>>);

pub type Kv = (Key, Vec<Word>);

impl State {
    // Empry state, fine for tests unrelated to reading state.
    pub const EMPTY: Self = State(BTreeMap::new());

    // Shorthand test state constructor.
    pub fn new(sets: Vec<(ContentAddress, Vec<Kv>)>) -> Self {
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
    pub fn set(&mut self, set_addr: ContentAddress, key: &Key, value: Vec<Word>) {
        let set = self.0.entry(set_addr).or_default();
        if value.is_empty() {
            set.remove(key);
        } else {
            set.insert(key.clone(), value);
        }
    }

    pub fn deploy_namespace(&mut self, set_addr: ContentAddress) {
        self.0.entry(set_addr).or_default();
    }

    /// Retrieve a word range.
    pub fn key_range(
        &self,
        set_addr: ContentAddress,
        mut key: Key,
        num_words: usize,
    ) -> Result<Vec<Vec<Word>>, String> {
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

        // If the intent does not exist yet, assume `None`s as though intent hasn't been deployed yet?
        let set = match self.get(&set_addr) {
            None => return Err("".to_string()),
            Some(set) => set,
        };

        // Collect the words.
        let mut words = vec![];
        for _ in 0..num_words {
            let opt = set.get(&key).cloned().unwrap_or_default();
            words.push(opt);
            key = next_key(key).ok_or("".to_string())?;
        }
        Ok(words)
    }

    /// Apply all mutations proposed by the given solution.
    pub fn apply_mutations(&mut self, solution: &Solution) {
        for state_mutation in &solution.state_mutations {
            let set = &solution
                .data
                .get(state_mutation.pathway as usize)
                .expect("intent pathway not found in solution data")
                .intent_to_solve
                .set;
            for mutation in state_mutation.mutations.iter() {
                self.set(set.clone(), &mutation.key, mutation.value.clone());
            }
        }
    }
}

impl core::ops::Deref for State {
    type Target = BTreeMap<ContentAddress, BTreeMap<Key, Vec<Word>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StateRead for State {
    type Error = String;
    type Future = Ready<Result<Vec<Vec<Word>>, Self::Error>>;
    fn key_range(&self, set_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        future::ready(self.key_range(set_addr, key, num_words))
    }
}

fn test_intent_42(entropy: Word) -> Intent {
    Intent {
        // State read program to read state slot 0.
        state_read: vec![state_read_vm::asm::to_bytes([
            state_read_vm::asm::Stack::Push(1).into(),
            state_read_vm::asm::StateSlots::AllocSlots.into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(4).into(),
            state_read_vm::asm::Stack::Push(1).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::StateRead::KeyRange,
            state_read_vm::asm::ControlFlow::Halt.into(),
        ])
        .collect()],
        // Program to check pre-mutation value is None and
        // post-mutation value is 42 at slot 0.
        constraints: vec![constraint_vm::asm::to_bytes([
            state_read_vm::asm::Stack::Push(entropy).into(),
            state_read_vm::asm::Stack::Pop.into(),
            constraint_vm::asm::Stack::Push(0).into(), // slot
            constraint_vm::asm::Stack::Push(0).into(), // pre
            constraint_vm::asm::Access::StateLen.into(),
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Pred::Eq.into(),
            constraint_vm::asm::Stack::Push(0).into(), // slot
            constraint_vm::asm::Stack::Push(1).into(), // post
            constraint_vm::asm::Access::State.into(),
            constraint_vm::asm::Stack::Push(42).into(),
            constraint_vm::asm::Pred::Eq.into(),
            constraint_vm::asm::Pred::And.into(),
        ])
        .collect()],
        directive: Directive::Satisfy,
    }
}

fn intent_set_addr(intents: &Signed<Vec<Intent>>) -> ContentAddress {
    essential_hash::intent_set_addr::from_intents(&intents.data)
}

fn intent_addr(intents: &Signed<Vec<Intent>>, ix: usize) -> IntentAddress {
    IntentAddress {
        set: intent_set_addr(intents),
        intent: ContentAddress(essential_hash::hash(&intents.data[ix])),
    }
}

fn random_keypair(seed: [u8; 32]) -> (SecretKey, PublicKey) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::SmallRng::from_seed(seed);
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut rng)
}

fn test_intent_42_solution_pair(
    entropy: Word,
    keypair_seed: [u8; 32],
) -> (Signed<Vec<Intent>>, Solution) {
    // Create the test intent, ensure its decision_variables match, and sign.
    let intent = test_intent_42(entropy);
    let (sk, _pk) = random_keypair(keypair_seed);
    let intents = essential_sign::sign(vec![intent], sk);
    let intent_addr = intent_addr(&intents, 0);

    // Construct the solution decision variables.
    // The first is an inline variable 42.
    let decision_variables = vec![42];

    // Create the solution.
    let solution = Solution {
        data: vec![SolutionData {
            intent_to_solve: intent_addr,
            decision_variables,
        }],
        transient_data: vec![],
        state_mutations: vec![Mutations {
            pathway: 0,
            mutations: vec![Mutation {
                key: vec![0, 0, 0, 0],
                value: vec![42],
            }],
        }],
    };

    (intents, solution)
}
