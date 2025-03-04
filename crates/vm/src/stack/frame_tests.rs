use super::*;

// Helper function to create a stack with initial values
fn setup_stack(values: &[Word]) -> Stack {
    let mut stack = Stack::default();
    for &value in values {
        stack.push(value).unwrap();
    }
    stack
}

#[test]
fn test_reserve_zeroed_success() {
    let mut stack = setup_stack(&[3]); // Push length 3
    assert!(stack.reserve_zeroed().is_ok());
    assert_eq!(stack.0, vec![0, 0, 0]);
    stack.push(2).unwrap(); // Push length 2
    assert!(stack.reserve_zeroed().is_ok());
    assert_eq!(stack.0, vec![0, 0, 0, 0, 0]);
}

#[test]
fn test_reserve_zeroed_empty_stack() {
    let mut stack = Stack::default();
    assert!(matches!(
        stack.reserve_zeroed().unwrap_err(),
        StackError::Empty
    ));
}

#[test]
fn test_reserve_zeroed_negative_length() {
    let mut stack = setup_stack(&[-1]);
    assert!(matches!(
        stack.reserve_zeroed().unwrap_err(),
        StackError::IndexOutOfBounds
    ));
}

#[test]
fn test_reserve_zeroed_exceeds_size_limit() {
    let mut stack = setup_stack(&[Word::MAX]);
    assert!(matches!(
        stack.reserve_zeroed().unwrap_err(),
        StackError::IndexOutOfBounds
    ));
}

#[test]
fn test_load_success() {
    let mut stack = setup_stack(&[42, 84, 126]);
    stack.push(1).unwrap(); // Push index 1
    assert!(stack.load().is_ok());
    assert_eq!(stack.0, vec![42, 84, 126, 84]);
    assert_eq!(stack.pop().unwrap(), 84); // Should load value at index 1
}

#[test]
fn test_load_empty_stack() {
    let mut stack = Stack::default();
    assert!(matches!(stack.load().unwrap_err(), StackError::Empty));
}

#[test]
fn test_load_negative_index() {
    let mut stack = setup_stack(&[-1]);
    assert!(matches!(
        stack.load().unwrap_err(),
        StackError::IndexOutOfBounds
    ));
}

#[test]
fn test_load_out_of_bounds() {
    let mut stack = setup_stack(&[5]);
    stack.0 = vec![1, 2, 3]; // Memory only has 3 elements
    assert!(matches!(
        stack.load().unwrap_err(),
        StackError::IndexOutOfBounds
    ));
}

#[test]
fn test_store_success() {
    let mut stack = setup_stack(&[1, 2, 3]);
    stack.push(99).unwrap(); // Value to store
    stack.push(1).unwrap(); // Index to store at
    assert!(stack.store().is_ok());
    assert_eq!(stack.0, vec![1, 99, 3]);
}

#[test]
fn test_store_empty_stack() {
    let mut stack = Stack::default();
    assert!(matches!(stack.store().unwrap_err(), StackError::Empty));
}

#[test]
fn test_store_single_value_stack() {
    let mut stack = setup_stack(&[42]);
    assert!(matches!(stack.store().unwrap_err(), StackError::Empty));
}

#[test]
fn test_store_negative_index() {
    let mut stack = setup_stack(&[99, -1]); // Value and negative index
    assert!(matches!(
        stack.store().unwrap_err(),
        StackError::IndexOutOfBounds
    ));
}

#[test]
fn test_store_out_of_bounds() {
    let mut stack = setup_stack(&[1, 2, 3]);
    stack.push(5).unwrap(); // Index out of bounds
    stack.push(99).unwrap(); // Value to store
    assert!(matches!(
        stack.store().unwrap_err(),
        StackError::IndexOutOfBounds
    ));
}
