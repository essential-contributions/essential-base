use essential_check::{
    solution::{self},
    vm::asm,
};
use essential_hash::content_addr;
use essential_types::{
    contract::Contract,
    predicate::{Edge, Node, Predicate, Program},
    solution::{Solution, SolutionSet},
    ContentAddress, PredicateAddress,
};
use std::{collections::HashMap, sync::Arc};
use util::State;

pub mod util;

#[test]
fn test_throughput() {
    use essential_vm::asm::short::*;
    let _ = tracing_subscriber::fmt::try_init();

    let short_n = std::env::var("NUM_SHORT_PROGRAMS")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap();
    let long_n = std::env::var("NUM_LONG_PROGRAMS")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap();

    for name in ["Short", "Long"].iter() {
        let p = match *name {
            "Short" => {
                Program(asm::to_bytes([PUSH(0), PUSH(0), PUSH(1), DATA, PUSH(1), GTE]).collect())
            }
            "Long" => Program(
                asm::to_bytes([
                    PUSH(1),
                    PUSH(5000),
                    PUSH(1),
                    REP,
                    PUSH(0),
                    PUSH(0),
                    PUSH(1),
                    DATA,
                    PUSH(1),
                    GTE,
                    AND,
                    REPE,
                ])
                .collect(),
            ),
            _ => unreachable!(),
        };

        let n = match *name {
            "Short" => short_n,
            "Long" => long_n,
            _ => unreachable!(),
        };

        let p_ca = content_addr(&p);

        let node = |program_address, edge_start| Node {
            program_address,
            edge_start,
        };
        let nodes = vec![node(p_ca.clone(), Edge::MAX)];
        let edges = vec![];
        let predicate_0 = Predicate { nodes, edges };
        let contract_0 = Contract::without_salt(vec![predicate_0]);
        let pred_addr_0 = PredicateAddress {
            contract: content_addr(&contract_0),
            predicate: content_addr(&contract_0.predicates[0]),
        };

        let set = SolutionSet {
            solutions: vec![Solution {
                predicate_to_solve: pred_addr_0.clone(),
                predicate_data: vec![vec![2]],
                state_mutations: vec![],
            }],
        };

        let predicate_0 = Arc::new(contract_0.predicates[0].clone());
        let mut map = HashMap::new();
        map.insert(pred_addr_0.contract.clone(), predicate_0);

        let get_predicate = |addr: &PredicateAddress| map.get(&addr.contract).unwrap().clone();
        let programs: HashMap<ContentAddress, Arc<Program>> =
            vec![(p_ca, Arc::new(p))].into_iter().collect();
        let get_program: Arc<HashMap<_, _>> = Arc::new(programs);

        let set = Arc::new(set);
        let config = Arc::new(solution::CheckPredicateConfig::default());
        let mut gas = 0;
        let s = std::time::Instant::now();
        for _ in 0..n {
            let outputs = solution::check_set_predicates(
                &State::EMPTY,
                set.clone(),
                get_predicate,
                get_program.clone(),
                config.clone(),
            )
            .unwrap();
            assert!(outputs.gas > 0);
            gas += outputs.gas;
        }
        let elapsed = s.elapsed();

        println!("Gas: {}", gas);
        println!("Elapsed: {:?}, per run: {:?}", elapsed, elapsed / n);
    }
}
