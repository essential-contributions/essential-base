use crate::solution::decode::{decode_mutation, decode_mutations};

use super::*;

#[test]
fn test_encode_mutation() {
    let m = Mutation {
        key: vec![1, 2, 3],
        value: vec![4, 5],
    };
    let words = encode_mutation(&m).collect::<Vec<_>>();
    assert_eq!(words, vec![3, 1, 2, 3, 2, 4, 5]);

    // Round trip
    let m2 = decode_mutation(&words).unwrap();
    assert_eq!(m, m2);
}

#[test]
fn test_encode_mutations() {
    let m = vec![
        Mutation {
            key: vec![1, 2, 3],
            value: vec![4, 5],
        },
        Mutation {
            key: vec![6, 7],
            value: vec![8, 9, 10],
        },
    ];

    let words = encode_mutations(&m).collect::<Vec<_>>();
    assert_eq!(words, vec![2, 3, 1, 2, 3, 2, 4, 5, 2, 6, 7, 3, 8, 9, 10]);

    // Round trip
    let m2 = decode_mutations(&words).unwrap();
    assert_eq!(m, m2);
}
