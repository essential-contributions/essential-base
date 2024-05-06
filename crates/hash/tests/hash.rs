use essential_types::{
    intent::{Directive, Intent},
    slots::Slots,
};

fn test_intent() -> Intent {
    Intent {
        slots: Slots {
            decision_variables: 1,
            state: Default::default(),
        },
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

#[test]
fn serialize_intent() {
    let serialization = essential_hash::serialize(&test_intent());
    let hex = hex::encode(serialization);
    let expected_hex = "0100000000";
    assert_eq!(hex, expected_hex);
}

#[test]
fn hash_intent() {
    let hash = essential_hash::hash(&test_intent());
    let expected_hash_hex = "957b88b12730e646e0f33d3618b77dfa579e8231e3c59c7104be7165611c8027";
    let hex = hex::encode(hash);
    assert_eq!(hex, expected_hash_hex);
}
