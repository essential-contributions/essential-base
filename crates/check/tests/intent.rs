use essential_check::intent;
use essential_sign::sign;
use essential_types::{
    intent::{Directive, Intent},
    slots::{Slots, StateSlot},
};
use util::{empty_intent, empty_state_slot, random_keypair};

pub mod util;

#[test]
fn signed_set_one_empty_intent() {
    let intent = empty_intent();
    let (sk, _pk) = random_keypair([0; 32]);
    let signed = sign(vec![intent], sk);
    intent::check_signed_set(&signed).unwrap();
}

#[test]
fn invalid_signature() {
    let intent = empty_intent();
    let (sk, _pk) = random_keypair([0; 32]);
    let mut signed = sign(vec![intent], sk);
    signed.signature.0 = [0; 64];
    assert!(matches!(
        intent::check_signed_set(&signed).unwrap_err(),
        intent::InvalidSignedSet::Signature(_),
    ));
}

#[test]
fn too_many_intents() {
    let intents: Vec<Intent> = vec![empty_intent(); intent::MAX_INTENTS + 1];
    let (sk, _pk) = random_keypair([0; 32]);
    let signed = sign(intents, sk);
    assert!(matches!(
        intent::check_signed_set(&signed).unwrap_err(),
        intent::InvalidSignedSet::Set(intent::InvalidSet::TooManyIntents(n))
            if n == intent::MAX_INTENTS + 1
    ));
}

#[test]
fn too_many_decision_variables() {
    let mut intent = empty_intent();
    intent.slots = Slots {
        decision_variables: intent::MAX_DECISION_VARIABLES + 1,
        state: Default::default(),
    };
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::InvalidIntent::Slots(intent::InvalidSlots::TooManyDecisionVariables(n))
            if n == intent::MAX_DECISION_VARIABLES + 1
    ));
}

#[test]
fn too_many_state_slots() {
    let mut intent = empty_intent();
    intent.slots = Slots {
        decision_variables: Default::default(),
        state: vec![empty_state_slot(); intent::MAX_NUM_STATE_SLOTS + 1],
    };
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::InvalidIntent::Slots(intent::InvalidSlots::TooManyStateSlots(n))
            if n == intent::MAX_NUM_STATE_SLOTS + 1
    ));
}

#[test]
fn state_slot_length_exceeds_limit() {
    let mut intent = empty_intent();
    intent.slots = Slots {
        decision_variables: Default::default(),
        state: vec![StateSlot {
            index: u32::MAX,
            amount: 1,
            program_index: Default::default(),
        }],
    };
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::InvalidIntent::Slots(intent::InvalidSlots::StateSlotLengthExceedsLimit(None)),
    ));
}

#[test]
fn test_fail_state_slots_length_too_large() {
    let mut intent = empty_intent();
    intent.slots = Slots {
        decision_variables: Default::default(),
        state: vec![StateSlot {
            index: Default::default(),
            amount: intent::MAX_STATE_LEN + 1,
            program_index: Default::default(),
        }],
    };
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::InvalidIntent::Slots(intent::InvalidSlots::StateSlotLengthExceedsLimit(Some(n)))
            if n == intent::MAX_STATE_LEN + 1,
    ));
}

#[test]
fn directive_too_large() {
    let mut intent = empty_intent();
    intent.directive = Directive::Maximize(vec![0; intent::MAX_DIRECTIVE_SIZE + 1]);
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::InvalidIntent::Directive(intent::InvalidDirective::TooManyBytes(n))
            if n == intent::MAX_DIRECTIVE_SIZE + 1
    ));
}

#[test]
fn too_many_state_reads() {
    let mut intent = empty_intent();
    intent.state_read = vec![vec![]; intent::MAX_STATE_READS + 1];
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::InvalidIntent::StateReads(intent::InvalidStateReads::TooMany(n))
            if n == intent::MAX_STATE_READS + 1
    ));
}

#[test]
fn state_read_too_large() {
    let mut intent = empty_intent();
    intent.state_read = vec![vec![0u8; intent::MAX_STATE_READ_SIZE_IN_BYTES + 1]];
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::InvalidIntent::StateReads(intent::InvalidStateReads::StateRead(0, intent::InvalidStateRead::TooManyBytes(n)))
            if n == intent::MAX_STATE_READ_SIZE_IN_BYTES + 1
    ));
}

#[test]
fn too_many_constraints() {
    let mut intent = empty_intent();
    intent.constraints = vec![vec![]; intent::MAX_CONSTRAINTS + 1];
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::InvalidIntent::Constraints(intent::InvalidConstraints::TooManyConstraints(n))
            if n == intent::MAX_CONSTRAINTS + 1
    ));
}

#[test]
fn constraint_too_large() {
    let mut intent = empty_intent();
    intent.constraints = vec![vec![0u8; intent::MAX_CONSTRAINT_SIZE_IN_BYTES + 1]];
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::InvalidIntent::Constraints(intent::InvalidConstraints::Constraint(0, intent::InvalidConstraint::TooManyBytes(n)))
            if n == intent::MAX_CONSTRAINT_SIZE_IN_BYTES + 1
    ));
}
