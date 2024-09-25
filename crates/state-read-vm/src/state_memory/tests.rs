use super::*;

#[test]
fn test_sm_alloc() {
    let mut slots = StateMemory::default();
    slots.alloc_slots(5000).unwrap_err();
    slots.alloc_slots(4000).unwrap();
    slots.alloc_slots(4000).unwrap_err();
}

#[test]
fn test_sm_load() {
    let mut slots = StateMemory::default();
    slots.load(0, 0..0).unwrap_err();
    slots.alloc_slots(1).unwrap();
    let r = slots.load(0, 0..0).unwrap();
    assert!(r.is_empty());
    slots.load(0, 0..1).unwrap_err();
    slots.store(0, 0, vec![1, 2, 3]).unwrap();
    let r = slots.load(0, 0..3).unwrap();
    assert_eq!(r, vec![1, 2, 3]);
    slots.load(0, 0..4).unwrap_err();
    slots.load(0, 3..4).unwrap_err();
    slots.load(1, 0..3).unwrap_err();
}

#[test]
fn test_sm_store() {
    let mut slots = StateMemory::default();
    slots.store(0, 0, vec![1, 2, 3]).unwrap_err();
    slots.alloc_slots(3).unwrap();
    slots.store(4, 0, vec![1, 2, 3]).unwrap_err();
    slots.store(2, 1, vec![1, 2, 3]).unwrap_err();
    slots.store(2, 0, vec![1, 2, 3]).unwrap();
    let r = slots.load(2, 0..3).unwrap();
    assert_eq!(r, vec![1, 2, 3]);
    slots.store(2, 4, vec![1, 2, 3]).unwrap_err();
    slots.store(2, 3, vec![1, 2, 3]).unwrap();
    let r = slots.load(2, 3..6).unwrap();
    assert_eq!(r, vec![1, 2, 3]);
    let r = slots.load(2, 0..6).unwrap();
    assert_eq!(r, vec![1, 2, 3, 1, 2, 3]);
}

#[test]
fn test_sm_truncate() {
    let mut slots = StateMemory::default();
    slots.truncate(0, 0).unwrap_err();
    slots.alloc_slots(3).unwrap();
    slots.truncate(0, 0).unwrap();
    slots.store(0, 0, vec![1, 2, 3]).unwrap();
    let r = slots.load(0, 0..3).unwrap();
    assert_eq!(r, vec![1, 2, 3]);
    slots.truncate(0, 2).unwrap();
    let r = slots.load(0, 0..2).unwrap();
    assert_eq!(r, vec![1, 2]);
    slots.load(0, 0..3).unwrap_err();

    slots.truncate(0, 0).unwrap();
    slots.load(0, 0..1).unwrap_err();
    let r = slots.load(0, 0..0).unwrap();
    assert!(r.is_empty());
}

#[test]
fn test_alloc() {
    let mut slots = StateMemory::default();
    let mut stack = Stack::default();

    alloc_slots(&mut stack, &mut slots).unwrap_err();

    stack.push(-10).unwrap();
    alloc_slots(&mut stack, &mut slots).unwrap_err();

    stack.push(1).unwrap();
    alloc_slots(&mut stack, &mut slots).unwrap();
    assert_eq!(slots.len(), 1);
}

#[test]
fn test_length() {
    let mut slots = StateMemory::default();
    let mut stack = Stack::default();

    length(&mut stack, &slots).unwrap();
    assert_eq!(stack.pop().unwrap(), 0);

    slots.alloc_slots(30).unwrap();
    length(&mut stack, &slots).unwrap();
    assert_eq!(stack.pop().unwrap(), 30);
}

#[test]
fn test_value_len() {
    let mut slots = StateMemory::default();
    let mut stack = Stack::default();

    value_len(&mut stack, &slots).unwrap_err();

    slots.alloc_slots(30).unwrap();

    stack.push(-1).unwrap();
    value_len(&mut stack, &slots).unwrap_err();

    slots.store(5, 0, vec![1, 2]).unwrap();
    stack.push(5).unwrap();
    value_len(&mut stack, &slots).unwrap();

    assert_eq!(stack.pop().unwrap(), 2);
}

#[test]
fn test_truncate() {
    let mut slots = StateMemory::default();
    let mut stack = Stack::default();

    truncate(&mut stack, &mut slots).unwrap_err();

    stack.push(0).unwrap();
    truncate(&mut stack, &mut slots).unwrap_err();

    slots.alloc_slots(30).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    truncate(&mut stack, &mut slots).unwrap();

    stack.push(-1).unwrap();
    stack.push(0).unwrap();
    truncate(&mut stack, &mut slots).unwrap_err();

    stack.push(0).unwrap();
    stack.push(-1).unwrap();
    truncate(&mut stack, &mut slots).unwrap_err();

    slots.store(5, 0, vec![1, 2]).unwrap();
    stack.push(5).unwrap();
    stack.push(1).unwrap();
    truncate(&mut stack, &mut slots).unwrap();
    assert_eq!(slots[5], &[1]);
}

#[test]
fn test_load() {
    let mut slots = StateMemory::default();
    let mut stack = Stack::default();

    slots.alloc_slots(30).unwrap();

    load(&mut stack, &slots).unwrap_err();

    stack.push(0).unwrap();
    load(&mut stack, &slots).unwrap_err();

    stack.push(0).unwrap();
    stack.push(0).unwrap();
    load(&mut stack, &slots).unwrap_err();

    stack.push(-1).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    load(&mut stack, &slots).unwrap_err();

    stack.push(0).unwrap();
    stack.push(-1).unwrap();
    stack.push(0).unwrap();
    load(&mut stack, &slots).unwrap_err();

    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(-1).unwrap();
    load(&mut stack, &slots).unwrap_err();

    slots.store(5, 0, (0..20).collect()).unwrap();
    stack.push(5).unwrap();
    stack.push(1).unwrap();
    stack.push(18).unwrap();
    load(&mut stack, &slots).unwrap();
    let r = stack.iter().copied().collect::<Vec<_>>();
    assert_eq!(r, (1..19).collect::<Vec<_>>());
}

#[test]
fn test_store() {
    let mut slots = StateMemory::default();

    slots.alloc_slots(30).unwrap();

    let mut stack = Stack::default();
    store(&mut stack, &mut slots).unwrap_err();

    let mut stack = Stack::default();
    stack.push(0).unwrap();
    store(&mut stack, &mut slots).unwrap_err();

    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    store(&mut stack, &mut slots).unwrap_err();

    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    store(&mut stack, &mut slots).unwrap_err();

    let mut stack = Stack::default();
    stack.push(-1).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    store(&mut stack, &mut slots).unwrap_err();

    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(-1).unwrap();
    stack.push(0).unwrap();
    stack.push(1).unwrap();
    store(&mut stack, &mut slots).unwrap_err();

    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(-1).unwrap();
    store(&mut stack, &mut slots).unwrap_err();

    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(99).unwrap();
    stack.push(2).unwrap();
    store(&mut stack, &mut slots).unwrap_err();

    let mut stack = Stack::default();
    stack.push(0).unwrap();
    stack.push(0).unwrap();
    stack.push(99).unwrap();
    stack.push(5).unwrap();
    store(&mut stack, &mut slots).unwrap_err();

    let mut stack = Stack::default();
    stack.push(5).unwrap();
    stack.push(0).unwrap();
    stack.push(99).unwrap();
    stack.push(99).unwrap();
    stack.push(2).unwrap();
    store(&mut stack, &mut slots).unwrap();
    stack.push(5).unwrap();
    stack.push(2).unwrap();
    stack.push(1).unwrap();
    stack.push(2).unwrap();
    stack.push(3).unwrap();
    stack.push(3).unwrap();
    store(&mut stack, &mut slots).unwrap();

    assert_eq!(slots[5], &[99, 99, 1, 2, 3]);
}
