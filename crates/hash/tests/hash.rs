use essential_types::predicate::{Directive, Predicate};

fn test_predicate() -> Predicate {
    Predicate {
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

#[test]
fn serialize_predicate() {
    let serialization = essential_hash::serialize(&test_predicate());
    let hex = hex::encode(serialization);
    let expected_hex = "000000";
    assert_eq!(hex, expected_hex);
}

#[test]
fn hash_predicate() {
    let hash = essential_hash::hash(&test_predicate());
    let expected_hash_hex = "709e80c88487a2411e1ee4dfb9f22a861492d20c4765150c0c794abd70f8147c";
    let hex = hex::encode(hash);
    assert_eq!(hex, expected_hash_hex);
}
