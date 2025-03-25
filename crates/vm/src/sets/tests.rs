use super::*;
use crate::error::OpError;
use std::collections::HashSet;

#[test]
fn test_encode_decode() {
    let set: HashSet<Vec<Word>> = [vec![42, 99], vec![21, 12, 8899, -72]]
        .into_iter()
        .collect();
    let mut stack = Stack::default();
    encode_set(set.iter().map(|i| i.iter().copied()), &mut stack).unwrap();

    // Pop total length.
    stack.pop().unwrap();

    let r = decode_set(&stack)
        .map(Result::unwrap)
        .map(Vec::from)
        .collect::<HashSet<_>>();
    assert_eq!(r, set);
}

#[test]
fn test_encode_set() {
    let items = [vec![-1, -2], vec![-3, -4, -5, -6]];
    let mut stack = Stack::default();
    encode_set(items.clone().into_iter().map(|i| i.into_iter()), &mut stack).unwrap();

    let total_len = stack.pop().unwrap();
    assert_eq!(total_len, 8);

    let item_1_len = stack.pop().unwrap();
    assert_eq!(item_1_len, items[1].len() as Word);
    let r = stack.pop4().unwrap();
    assert_eq!(r.as_slice(), items[1].as_slice());

    let item_0_len = stack.pop().unwrap();
    assert_eq!(item_0_len, items[0].len() as Word);
    let r = stack.pop2().unwrap();
    assert_eq!(r.as_slice(), items[0].as_slice());

    assert!(stack.is_empty());
}

#[test]
fn test_decode_set() {
    let set = [0, 1, 2, 3, 3, 4, 5, 3, 6, 1];
    let expect: [&[Word]; 3] = [&[0, 1, 2], &[3, 4, 5], &[6]];
    let lhs = decode_set(&set).collect::<Result<HashSet<_>, _>>().unwrap();
    let rhs = expect.into_iter().collect();
    assert_eq!(lhs, rhs);

    let set = [0, 1, 2, 4, 3, 4, 5, 3, 6, 1];
    let res = decode_set(&set).collect::<Result<Vec<_>, _>>();
    assert!(matches!(res.unwrap_err(), OpError::Decode(DecodeError::Set(s)) if s == set));
}
