use std::{
    collections::{BTreeMap, HashMap},
    future::{self, Ready},
    sync::Arc,
};

use criterion::{criterion_group, criterion_main, Criterion};
use essential_constraint_vm as constraint_vm;
use essential_state_read_vm as state_read_vm;
use essential_state_read_vm::StateRead;
use essential_types::{
    contract::{Contract, SignedContract},
    predicate::Predicate,
    solution::{Mutation, Solution, SolutionData},
    ContentAddress, Key, PredicateAddress, Word,
};
use secp256k1::{PublicKey, Secp256k1, SecretKey};

pub fn bench(c: &mut Criterion) {
    let config = Arc::new(essential_check::solution::CheckPredicateConfig::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    for i in [1, 10, 100, 1000, 10_000] {
        let (predicates, solution, predicates_map) = create(i);
        let get_predicate = |addr: &PredicateAddress| predicates_map.get(addr).cloned().unwrap();
        let mut pre_state = State::EMPTY;
        pre_state.deploy_namespace(essential_hash::content_addr(&predicates.contract));
        let mut post_state = pre_state.clone();
        post_state.apply_mutations(&solution);
        c.bench_function(&format!("check_42_{}", i), |b| {
            b.to_async(&runtime).iter(|| {
                essential_check::solution::check_predicates(
                    &pre_state,
                    &post_state,
                    solution.clone(),
                    get_predicate,
                    config.clone(),
                )
            });
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);

#[allow(clippy::type_complexity)]
fn create(
    amount: usize,
) -> (
    SignedContract,
    Arc<Solution>,
    HashMap<PredicateAddress, Arc<Predicate>>,
) {
    let (predicates, solution) = test_predicate_42_solution_pair(amount, [0; 32]);
    let contract = contract_addr(&predicates);
    let predicates_map: HashMap<_, _> = predicates
        .contract
        .iter()
        .map(|predicate| {
            (
                PredicateAddress {
                    contract: contract.clone(),
                    predicate: essential_hash::content_addr(predicate),
                },
                Arc::new(predicate.clone()),
            )
        })
        .collect();
    (predicates, Arc::new(solution), predicates_map)
}

#[derive(Clone, Debug)]
pub struct State(BTreeMap<ContentAddress, BTreeMap<Key, Vec<Word>>>);

pub type Kv = (Key, Vec<Word>);

impl State {
    // Empry state, fine for tests unrelated to reading state.
    pub const EMPTY: Self = State(BTreeMap::new());

    // Shorthand test state constructor.
    pub fn new(contracts: Vec<(ContentAddress, Vec<Kv>)>) -> Self {
        State(
            contracts
                .into_iter()
                .map(|(addr, vec)| {
                    let map: BTreeMap<_, _> = vec.into_iter().collect();
                    (addr, map)
                })
                .collect(),
        )
    }

    // Update the value at the given key within the given contract address.
    pub fn set(&mut self, contract_addr: ContentAddress, key: &Key, value: Vec<Word>) {
        let contract = self.0.entry(contract_addr).or_default();
        if value.is_empty() {
            contract.remove(key);
        } else {
            contract.insert(key.clone(), value);
        }
    }

    pub fn deploy_namespace(&mut self, contract_addr: ContentAddress) {
        self.0.entry(contract_addr).or_default();
    }

    /// Retrieve a word range.
    pub fn key_range(
        &self,
        contract_addr: ContentAddress,
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

        // If the predicate does not exist yet, assume `None`s as though predicate hasn't been deployed yet?
        let contract = match self.get(&contract_addr) {
            None => return Err("".to_string()),
            Some(contract) => contract,
        };

        // Collect the words.
        let mut words = vec![];
        for _ in 0..num_words {
            let opt = contract.get(&key).cloned().unwrap_or_default();
            words.push(opt);
            key = next_key(key).ok_or("".to_string())?;
        }
        Ok(words)
    }

    /// Apply all mutations proposed by the given solution.
    pub fn apply_mutations(&mut self, solution: &Solution) {
        for data in &solution.data {
            for mutation in &data.state_mutations {
                self.set(
                    data.predicate_to_solve.contract.clone(),
                    &mutation.key,
                    mutation.value.clone(),
                );
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
    fn key_range(&self, contract_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        future::ready(self.key_range(contract_addr, key, num_words))
    }
}

fn test_predicate_42(entropy: Word) -> Predicate {
    Predicate {
        // State read program to read state slot 0.
        state_read: vec![state_read_vm::asm::to_bytes([
            state_read_vm::asm::Stack::Push(1).into(),
            state_read_vm::asm::StateMemory::AllocSlots.into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::Stack::Push(4).into(),
            state_read_vm::asm::Stack::Push(1).into(),
            state_read_vm::asm::Stack::Push(0).into(),
            state_read_vm::asm::StateRead::KeyRange,
            state_read_vm::asm::TotalControlFlow::Halt.into(),
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
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(0).into(), // slot
            constraint_vm::asm::Stack::Push(1).into(), // post
            constraint_vm::asm::Access::State.into(),
            constraint_vm::asm::Stack::Push(42).into(),
            constraint_vm::asm::Pred::Eq.into(),
            constraint_vm::asm::Pred::And.into(),
        ])
        .collect()],
    }
}

fn contract_addr(contract: &SignedContract) -> ContentAddress {
    essential_hash::content_addr(&contract.contract)
}

fn random_keypair(seed: [u8; 32]) -> (SecretKey, PublicKey) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::SmallRng::from_seed(seed);
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut rng)
}

fn test_predicate_42_solution_pair(
    amount: usize,
    keypair_seed: [u8; 32],
) -> (SignedContract, Solution) {
    // Create the test predicate, ensure its decision_variables match, and sign.
    let predicates: Vec<_> = (0..amount).map(|i| test_predicate_42(i as Word)).collect();
    let contract = Contract::without_salt(predicates);
    let (sk, _pk) = random_keypair(keypair_seed);
    let signed_contract = essential_sign::contract::sign(contract, &sk);

    let contract_addr = contract_addr(&signed_contract);

    let data = (0..amount)
        .map(|i| SolutionData {
            predicate_to_solve: PredicateAddress {
                contract: contract_addr.clone(),
                predicate: essential_hash::content_addr(signed_contract.contract.get(i).unwrap()),
            },
            decision_variables: vec![vec![42]],
            state_mutations: vec![Mutation {
                key: vec![0, 0, 0, 0],
                value: vec![42],
            }],
        })
        .collect();

    let solution = Solution { data };

    (signed_contract, solution)
}
