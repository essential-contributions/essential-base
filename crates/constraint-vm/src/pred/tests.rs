use super::*;

#[test]
fn test_eq_range_true() {
    let mut stack = Stack::default();
    stack.extend([1, 2, 3]).unwrap();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(3).unwrap();
    eq_range(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 1);
}

#[test]
fn test_eq_range_false() {
    let mut stack = Stack::default();
    stack.extend([1, 4, 3]).unwrap();
    stack.extend([1, 2, 3]).unwrap();
    stack.push(3).unwrap();
    eq_range(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);
}

#[test]
fn test_eq_empty_range() {
    let mut stack = Stack::default();
    stack.push(0).unwrap();
    eq_range(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 1);
}

#[test]
fn test_decode_set() {
    let set = [0, 1, 2, 3, 3, 4, 5, 3, 6, 1];
    let expect: [&[Word]; 3] = [&[0, 1, 2], &[3, 4, 5], &[6]];
    assert_eq!(decode_set(&set).unwrap(), expect.into_iter().collect());

    let set = [0, 1, 2, 4, 3, 4, 5, 3, 6, 1];
    assert!(
        matches!(decode_set(&set).unwrap_err(), OpError::Decode(DecodeError::Set(s)) if s == set)
    );
}

#[test]
fn test_eq_set() {
    let set_a = [0, 1, 2, 3, 3, 4, 5, 3, 6, 1, 10];
    let set_b = set_a;
    let mut stack = Stack::default();

    // Equal sets.
    stack.extend(set_a).unwrap();
    stack.extend(set_b).unwrap();
    eq_set(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 1);
    assert!(stack.is_empty());

    // Unequal sets of the same length.
    let set_c = [1, 1, 2, 3, 3, 4, 5, 3, 6, 1, 10];
    stack.extend(set_a).unwrap();
    stack.extend(set_c).unwrap();
    eq_set(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);
    assert!(stack.is_empty());

    // Order doesn't matter.
    stack.extend(set_c).unwrap();
    stack.extend(set_a).unwrap();
    eq_set(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);
    assert!(stack.is_empty());

    let set_d = [1, 1, 2, 3, 4];

    // Unequal sets of different lengths.
    stack.extend(set_c).unwrap();
    stack.extend(set_d).unwrap();
    eq_set(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);
    assert!(stack.is_empty());

    // Empty set.
    let empty_set = [0];
    stack.extend(empty_set).unwrap();
    stack.extend(empty_set).unwrap();
    eq_set(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 1);
    assert!(stack.is_empty());

    // Empty set and non-empty set.
    stack.extend(empty_set).unwrap();
    stack.extend(set_a).unwrap();
    eq_set(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);
    assert!(stack.is_empty());

    // Order doesn't matter.
    stack.extend(set_a).unwrap();
    stack.extend(empty_set).unwrap();
    eq_set(&mut stack).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);
    assert!(stack.is_empty());

    // Decode error lhs.
    let set_err = [0, 1, 2, 3, 3, 4, 5, 9, 6, 1, 10];
    stack.extend(set_err).unwrap();
    stack.extend(set_a).unwrap();
    let e = eq_set(&mut stack).unwrap_err();
    assert!(matches!(e, OpError::Decode(DecodeError::Set(s)) if s == set_err[..10]));

    // Decode error rhs.
    let set_err = [0, 1, 2, 3, 3, 4, 5, 9, 6, 1, 10];
    stack.extend(set_a).unwrap();
    stack.extend(set_err).unwrap();
    let e = eq_set(&mut stack).unwrap_err();
    assert!(matches!(e, OpError::Decode(DecodeError::Set(s)) if s == set_err[..10]));

    // Decode error both.
    let set_err = [0, 1, 2, 3, 3, 4, 5, 9, 6, 1, 10];
    stack.extend(set_err).unwrap();
    stack.extend(set_err).unwrap();
    let e = eq_set(&mut stack).unwrap_err();
    assert!(matches!(e, OpError::Decode(DecodeError::Set(s)) if s == set_err[..10]));
}
