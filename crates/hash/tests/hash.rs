use essential_types::{
    contract::Contract,
    predicate::OldPredicate,
    solution::{Solution, SolutionData},
    Block, ContentAddress, PredicateAddress,
};
use sha2::Digest;

fn test_predicate() -> OldPredicate {
    OldPredicate {
        state_read: Default::default(),
        constraints: Default::default(),
    }
}

#[test]
fn serialize_predicate() {
    let serialization = essential_hash::serialize(&test_predicate());
    let hex = hex::encode(serialization);
    let expected_hex = "0000";
    assert_eq!(hex, expected_hex);
}

#[test]
fn hash_predicate() {
    let hash = essential_hash::hash(&test_predicate());
    let expected_hash_hex = "96a296d224f285c67bee93c30f8a309157f0daa35dc5b87e410b78630a09cfc7";
    let hex = hex::encode(hash);
    assert_eq!(hex, expected_hash_hex);
}

#[test]
fn test_content_addr() {
    let pred = &test_predicate();
    let header = pred.encoded_header().unwrap();
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    hasher.update(header.fixed_size_header.0);
    hasher.update(header.lens);
    for item in pred.programs() {
        hasher.update(item);
    }
    let addr = ContentAddress(hasher.finalize().into());
    let content_addr = essential_hash::content_addr(&test_predicate());
    assert_eq!(content_addr, addr);

    let contract = Contract {
        salt: Default::default(),
        predicates: vec![test_predicate()],
    };
    let addr = essential_hash::contract_addr::from_contract(&contract);
    let content_addr = essential_hash::content_addr(&contract);
    assert_eq!(content_addr, addr);

    let solution = Solution { data: vec![] };
    let addr = essential_hash::hash(&solution);
    let content_addr = essential_hash::content_addr(&solution);
    assert_eq!(content_addr.0, addr);

    let solutions = vec![
        Solution {
            data: vec![SolutionData {
                predicate_to_solve: PredicateAddress {
                    contract: ContentAddress([1; 32]),
                    predicate: ContentAddress([1; 32]),
                },
                decision_variables: Default::default(),
                transient_data: Default::default(),
                state_mutations: Default::default(),
            }],
        },
        Solution {
            data: vec![SolutionData {
                predicate_to_solve: PredicateAddress {
                    contract: ContentAddress([2; 32]),
                    predicate: ContentAddress([2; 32]),
                },
                decision_variables: Default::default(),
                transient_data: Default::default(),
                state_mutations: Default::default(),
            }],
        },
    ];
    let block = Block {
        number: 0,
        timestamp: core::time::Duration::from_secs(0),
        solutions: solutions.clone(),
    };
    let addr = essential_hash::block_addr::from_block(&block);
    let content_addr = essential_hash::content_addr(&block);
    assert_eq!(content_addr, addr);

    let solution_addrs = solutions.iter().rev().map(essential_hash::content_addr);
    let addr = essential_hash::block_addr::from_block_and_solutions_addrs(&block, solution_addrs);
    assert_ne!(content_addr, addr);
}
