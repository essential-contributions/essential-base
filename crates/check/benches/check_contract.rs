use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use criterion::{criterion_group, criterion_main, Criterion};
use essential_hash::content_addr;
use essential_types::{
    contract::{Contract, SignedContract},
    predicate::{Edge, Node, Predicate, Program},
    solution::{Mutation, Solution, SolutionSet},
    ContentAddress, Key, PredicateAddress, Word,
};
use essential_vm::{StateRead, StateReads};
use secp256k1::{PublicKey, Secp256k1, SecretKey};

pub fn bench(c: &mut Criterion) {
    let config = Arc::new(essential_check::solution::CheckPredicateConfig::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    for i in [1, 10, 100, 1000, 10_000] {
        let (contract, solution, predicates, programs) = create(i);
        let mut state = State::EMPTY;
        state.deploy_namespace(essential_hash::content_addr(&contract.contract));
        c.bench_function(&format!("check_42_{}", i), |b| {
            b.to_async(&runtime).iter(|| async {
                essential_check::solution::check_set_predicates(
                    &state,
                    solution.clone(),
                    predicates.clone(),
                    programs.clone(),
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
    Arc<SolutionSet>,
    Arc<HashMap<PredicateAddress, Arc<Predicate>>>,
    Arc<HashMap<ContentAddress, Arc<Program>>>,
) {
    let (contract, programs, solution) = test_predicate_42_solution_pair(amount, [0; 32]);
    let contract_ca = content_addr(&contract.contract);
    let predicates: HashMap<_, _> = contract
        .contract
        .predicates
        .iter()
        .map(|predicate| {
            (
                PredicateAddress {
                    contract: contract_ca.clone(),
                    predicate: essential_hash::content_addr(predicate),
                },
                Arc::new(predicate.clone()),
            )
        })
        .collect();
    (
        contract,
        Arc::new(solution),
        Arc::new(predicates),
        Arc::new(programs),
    )
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
    pub fn apply_mutations(&mut self, solution: &SolutionSet) {
        for data in &solution.solutions {
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
    fn key_range(
        &self,
        contract_addr: ContentAddress,
        key: Key,
        num_words: usize,
    ) -> Result<Vec<Vec<Word>>, Self::Error> {
        self.key_range(contract_addr, key, num_words)
    }
}

impl StateReads for State {
    type Error = String;
    type Pre = State;
    type Post = State;

    fn pre(&self) -> &Self {
        self // Assuming `State` itself represents the pre-state
    }

    fn post(&self) -> &Self {
        self // Assuming `State` itself represents the post-state
    }
}

fn test_predicate_42(entropy: Word) -> (HashMap<ContentAddress, Arc<Program>>, Predicate) {
    use essential_vm::asm::{self, short::*};

    // Program to read key [0, 0, 0, 0].
    let a = Program(
        asm::to_bytes([
            PUSH(1),
            PUSH(0),
            PUSH(0),
            PUSH(0),
            PUSH(0),
            PUSH(4),
            PUSH(1),
            PUSH(0),
            KRNG,
            // Read the value from "state" memory onto the stack.
            PUSH(0),
            PUSH(0),
            PUSH(1),
            LODS,
            HLT,
        ])
        .collect(),
    );

    // Program to check pre-mutation value is None and
    // post-mutation value is 42 at slot 0.
    let b = Program(
        asm::to_bytes([
            PUSH(entropy),
            POP,
            // Check the pre-state is 0, and the post state is 42.
            // We'll do this with `EqRange`.
            // First, push the `0`.
            PUSH(0),
            // Next retrieve the `42` from the predicate data.
            PUSH(0), // slot_ix
            PUSH(0), // value_ix
            PUSH(1), // len
            DATA,
            // Now EqRange.
            PUSH(2),
            EQRA,
        ])
        .collect(),
    );

    let a_ca = content_addr(&a);
    let b_ca = content_addr(&b);

    let node = |program_address, edge_start| Node {
        program_address,
        edge_start,
    };
    let nodes = vec![
        node(a_ca.clone(), 0),
        node(a_ca.clone(), 1),
        node(b_ca.clone(), Edge::MAX),
    ];
    let edges = vec![2, 2];

    let predicate = Predicate { nodes, edges };
    let programs = vec![(a_ca, Arc::new(a)), (b_ca, Arc::new(b))]
        .into_iter()
        .collect();

    (programs, predicate)
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
) -> (
    SignedContract,
    HashMap<ContentAddress, Arc<Program>>,
    SolutionSet,
) {
    // Create the test predicate, ensure its predicate_data matches, and sign.
    let (programs, predicates): (Vec<_>, _) =
        (0..amount).map(|i| test_predicate_42(i as Word)).unzip();
    let contract = Contract::without_salt(predicates);
    let (sk, _pk) = random_keypair(keypair_seed);
    let signed_contract = essential_sign::contract::sign(contract, &sk);

    let contract_addr = contract_addr(&signed_contract);

    let solutions = (0..amount)
        .map(|i| Solution {
            predicate_to_solve: PredicateAddress {
                contract: contract_addr.clone(),
                predicate: essential_hash::content_addr(signed_contract.contract.get(i).unwrap()),
            },
            predicate_data: vec![vec![42]],
            state_mutations: vec![Mutation {
                key: vec![0, 0, 0, 0],
                value: vec![42],
            }],
        })
        .collect();

    let set = SolutionSet { solutions };
    let programs = programs
        .into_iter()
        .flat_map(|map| map.into_iter())
        .collect();

    (signed_contract, programs, set)
}
