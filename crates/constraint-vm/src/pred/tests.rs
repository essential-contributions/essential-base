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
