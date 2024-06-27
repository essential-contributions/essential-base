use essential_check::predicate;
use essential_types::predicate::Directive;
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
fn directive_too_large() {
    let mut predicate = empty_predicate();
    predicate.directive = Directive::Maximize(vec![0; predicate::MAX_DIRECTIVE_SIZE + 1]);
    assert!(matches!(
        predicate::check(&predicate).unwrap_err(),
        predicate::InvalidPredicate::Directive(predicate::InvalidDirective::TooManyBytes(n))
            if n == predicate::MAX_DIRECTIVE_SIZE + 1
    ));
}

#[test]
fn too_many_state_reads() {
    let mut predicate = empty_predicate();
    predicate.state_read = vec![vec![]; predicate::MAX_STATE_READS + 1];
    assert!(matches!(
        predicate::check(&predicate).unwrap_err(),
        predicate::InvalidPredicate::StateReads(predicate::InvalidStateReads::TooMany(n))
            if n == predicate::MAX_STATE_READS + 1
    ));
}

#[test]
fn state_read_too_large() {
    let mut predicate = empty_predicate();
    predicate.state_read = vec![vec![0u8; predicate::MAX_STATE_READ_SIZE_IN_BYTES + 1]];
    assert!(matches!(
        predicate::check(&predicate).unwrap_err(),
        predicate::InvalidPredicate::StateReads(predicate::InvalidStateReads::StateRead(0, predicate::InvalidStateRead::TooManyBytes(n)))
            if n == predicate::MAX_STATE_READ_SIZE_IN_BYTES + 1
    ));
}

#[test]
fn too_many_constraints() {
    let mut predicate = empty_predicate();
    predicate.constraints = vec![vec![]; predicate::MAX_CONSTRAINTS + 1];
    assert!(matches!(
        predicate::check(&predicate).unwrap_err(),
        predicate::InvalidPredicate::Constraints(predicate::InvalidConstraints::TooManyConstraints(n))
            if n == predicate::MAX_CONSTRAINTS + 1
    ));
}

#[test]
fn constraint_too_large() {
    let mut predicate = empty_predicate();
    predicate.constraints = vec![vec![0u8; predicate::MAX_CONSTRAINT_SIZE_IN_BYTES + 1]];
    assert!(matches!(
        predicate::check(&predicate).unwrap_err(),
        predicate::InvalidPredicate::Constraints(predicate::InvalidConstraints::Constraint(0, predicate::InvalidConstraint::TooManyBytes(n)))
            if n == predicate::MAX_CONSTRAINT_SIZE_IN_BYTES + 1
    ));
}
