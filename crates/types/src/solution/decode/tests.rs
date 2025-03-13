use super::*;

#[test]
fn test_decode_mutation() {
    let words = vec![3, 1, 2, 3, 2, 4, 5];
    let m = decode_mutation(&words).unwrap();
    assert_eq!(m.key, vec![1, 2, 3]);
    assert_eq!(m.value, vec![4, 5]);

    let words = vec![4, 1, 2, 3, 2, 4, 5];
    decode_mutation(&words).expect_err("Value length too short");
}

#[test]
fn test_decode_mutations() {
    let words = vec![2, 3, 1, 2, 3, 2, 4, 5, 2, 6, 7, 3, 8, 9, 10];
    let m = decode_mutations(&words).unwrap();
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].key, vec![1, 2, 3]);
    assert_eq!(m[0].value, vec![4, 5]);
    assert_eq!(m[1].key, vec![6, 7]);
    assert_eq!(m[1].value, vec![8, 9, 10]);
}
