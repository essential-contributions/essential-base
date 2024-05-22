use essential_types::intent::{Directive, Intent};

fn test_intent() -> Intent {
    Intent {
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

#[test]
fn serialize_intent() {
    let serialization = essential_hash::serialize(&test_intent());
    let hex = hex::encode(serialization);
    let expected_hex = "000000";
    assert_eq!(hex, expected_hex);
}

#[test]
fn hash_intent() {
    let hash = essential_hash::hash(&test_intent());
    let expected_hash_hex = "709e80c88487a2411e1ee4dfb9f22a861492d20c4765150c0c794abd70f8147c";
    let hex = hex::encode(hash);
    assert_eq!(hex, expected_hash_hex);
}
