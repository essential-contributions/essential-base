use essential_check::predicate::{self, InvalidPredicate};
use essential_types::{
    predicate::{Node, Predicate, Reads},
    ContentAddress,
};
use util::{empty_predicate, random_keypair};

pub mod util;

#[test]
fn signed_contract_one_empty_predicate() {
    let (sk, _pk) = random_keypair([0; 32]);
    let signed = essential_sign::contract::sign(vec![empty_predicate()].into(), &sk);
    predicate::check_signed_contract(&signed).unwrap();
}

#[test]
fn invalid_signature() {
    let (sk, _pk) = random_keypair([0; 32]);
    let mut signed = essential_sign::contract::sign(vec![empty_predicate()].into(), &sk);
    signed.signature.0 = [0; 64];
    assert!(matches!(
        predicate::check_signed_contract(&signed).unwrap_err(),
        predicate::InvalidSignedContract::Signature(_),
    ));
}

#[test]
fn too_many_predicates() {
    let predicates: Vec<_> = vec![empty_predicate(); predicate::MAX_PREDICATES + 1];
    let (sk, _pk) = random_keypair([0; 32]);
    let signed = essential_sign::contract::sign(predicates.into(), &sk);
    assert!(matches!(
        predicate::check_signed_contract(&signed).unwrap_err(),
        predicate::InvalidSignedContract::Set(predicate::InvalidContract::TooManyPredicates(n))
            if n == predicate::MAX_PREDICATES + 1
    ));
}

#[test]
fn too_many_nodes() {
    let mut predicate = empty_predicate();
    predicate.nodes = vec![
        Node {
            edge_start: 0,
            program_address: ContentAddress([0; 32]),
            reads: Reads::Pre
        };
        usize::from(Predicate::MAX_NODES) + 1
    ];
    assert!(matches!(
        predicate::check(&predicate).unwrap_err(),
        InvalidPredicate::TooManyNodes(n)
            if n == usize::from(Predicate::MAX_NODES) + 1
    ));
}

#[test]
fn too_many_edges() {
    let mut predicate = empty_predicate();
    predicate.edges = vec![0; usize::from(Predicate::MAX_EDGES) + 1];
    assert!(matches!(
        predicate::check(&predicate).unwrap_err(),
        InvalidPredicate::TooManyEdges(n)
            if n == usize::from(Predicate::MAX_EDGES) + 1
    ));
}
