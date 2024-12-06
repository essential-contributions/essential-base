use essential_hash::hash_bytes;
use essential_types::{contract::Contract, predicate::Predicate, ContentAddress};

fn test_predicate() -> Predicate {
    Predicate::default()
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
    let bytes = pred.encode().unwrap();
    let bytes: Vec<_> = bytes.collect();
    let addr = ContentAddress(hash_bytes(&bytes));
    let content_addr = essential_hash::content_addr(&test_predicate());
    assert_eq!(content_addr, addr);

    let contract = Contract {
        salt: Default::default(),
        predicates: vec![test_predicate()],
    };
    let addr = essential_hash::contract_addr::from_contract(&contract);
    let content_addr = essential_hash::content_addr(&contract);
    assert_eq!(content_addr, addr);
}
