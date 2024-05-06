use essential_check::intent;
use essential_sign::{
    secp256k1::{PublicKey, Secp256k1, SecretKey},
    sign,
};
use essential_types::{
    intent::{Directive, Intent},
    slots::{Slots, StateSlot},
};

fn empty_intent() -> Intent {
    Intent {
        slots: Default::default(),
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

fn empty_state_slot() -> StateSlot {
    StateSlot {
        amount: Default::default(),
        index: Default::default(),
        program_index: Default::default(),
    }
}

fn random_keypair(seed: [u8; 32]) -> (SecretKey, PublicKey) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::SmallRng::from_seed(seed);
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut rng)
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
        intent::IntentError::Slots(intent::SlotsError::TooManyDecisionVariables(n))
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
        intent::IntentError::Slots(intent::SlotsError::TooManyStateSlots(n))
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
        intent::IntentError::Slots(intent::SlotsError::StateSlotLengthExceedsLimit(None)),
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
        intent::IntentError::Slots(intent::SlotsError::StateSlotLengthExceedsLimit(Some(n)))
            if n == intent::MAX_STATE_LEN + 1,
    ));
}

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
        intent::SignedSetError::InvalidSignature(_),
    ));
}

#[test]
fn too_many_intents() {
    let intents: Vec<Intent> = vec![empty_intent(); intent::MAX_INTENTS + 1];
    let (sk, _pk) = random_keypair([0; 32]);
    let signed = sign(intents, sk);
    assert!(matches!(
        intent::check_signed_set(&signed).unwrap_err(),
        intent::SignedSetError::InvalidSet(intent::SetError::TooManyIntents(n))
            if n == intent::MAX_INTENTS + 1
    ));
}

#[test]
fn directive_too_large() {
    let mut intent = empty_intent();
    intent.directive = Directive::Maximize(vec![0; intent::MAX_DIRECTIVE_SIZE + 1]);
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::IntentError::Directive(intent::DirectiveError::TooManyBytes(n))
            if n == intent::MAX_DIRECTIVE_SIZE + 1
    ));
}

#[test]
fn too_many_state_reads() {
    let mut intent = empty_intent();
    intent.state_read = vec![vec![]; intent::MAX_STATE_READS + 1];
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::IntentError::StateReads(intent::StateReadsError::TooMany(n))
            if n == intent::MAX_STATE_READS + 1
    ));
}

#[test]
fn state_read_too_large() {
    let mut intent = empty_intent();
    intent.state_read = vec![vec![0u8; intent::MAX_STATE_READ_SIZE_IN_BYTES + 1]];
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::IntentError::StateReads(intent::StateReadsError::StateRead(0, intent::StateReadError::TooManyBytes(n)))
            if n == intent::MAX_STATE_READ_SIZE_IN_BYTES + 1
    ));
}

#[test]
fn test_fail_too_many_constraints() {
    let mut intent = empty_intent();
    intent.constraints = vec![vec![]; intent::MAX_CONSTRAINTS + 1];
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::IntentError::Constraints(intent::ConstraintsError::TooManyConstraints(n))
            if n == intent::MAX_CONSTRAINTS + 1
    ));
}

#[test]
fn test_fail_constraint_too_large() {
    let mut intent = empty_intent();
    intent.constraints = vec![vec![0u8; intent::MAX_CONSTRAINT_SIZE_IN_BYTES + 1]];
    assert!(matches!(
        intent::check(&intent).unwrap_err(),
        intent::IntentError::Constraints(intent::ConstraintsError::Constraint(0, intent::ConstraintError::TooManyBytes(n)))
            if n == intent::MAX_CONSTRAINT_SIZE_IN_BYTES + 1
    ));
}
