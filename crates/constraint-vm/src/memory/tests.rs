use super::*;

#[test]
fn test_memory_store_load() {
    let mut memory = Memory::new();
    memory.load(0).unwrap_err();
    memory.store(0, 0).unwrap_err();

    memory.alloc(1).unwrap();
    assert_eq!(memory.load(0).unwrap(), 0);
    memory.store(0, 1).unwrap();
    assert_eq!(memory.load(0).unwrap(), 1);

    memory.load(1).unwrap_err();
    memory.store(1, 0).unwrap_err();

    assert_eq!(memory.len().unwrap(), 1);
}

#[test]
fn test_free_empty_memory() {
    let mut memory = Memory::new();
    assert!(memory.is_empty());

    // Trying to free address 0 from empty memory should fail
    assert!(matches!(
        memory.free(0),
        Err(TemporaryError::IndexOutOfBounds)
    ));
}

#[test]
fn test_free_valid_address() {
    let mut memory = Memory::new();

    // Allocate 10 words
    memory.alloc(10).unwrap();
    assert_eq!(memory.len().unwrap(), 10);

    // Fill memory with values
    for i in 0..10 {
        memory.store(i, i as Word).unwrap();
    }

    // Free from index 5
    memory.free(5).unwrap();

    // Verify new length
    assert_eq!(memory.len().unwrap(), 5);

    // Verify remaining values are intact
    for i in 0..5 {
        assert_eq!(memory.load(i).unwrap(), i as Word);
    }

    // Verify accessing freed memory fails
    assert!(matches!(
        memory.load(5),
        Err(TemporaryError::IndexOutOfBounds)
    ));
}

#[test]
fn test_free_at_last_index() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Free at the last valid index
    memory.free(4).unwrap();
    assert_eq!(memory.len().unwrap(), 4);

    // Verify the rest of the memory is intact
    for i in 0..4 {
        assert_eq!(memory.load(i).unwrap(), 0);
    }
}

#[test]
fn test_free_at_start() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Free at index 0
    memory.free(0).unwrap();
    assert!(memory.is_empty());

    // Verify all memory is freed
    assert_eq!(memory.len().unwrap(), 0);
}

#[test]
fn test_free_invalid_address() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Test with out of bounds index
    assert!(matches!(
        memory.free(5),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Test with very large index
    assert!(matches!(
        memory.free(Word::MAX),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Verify memory state hasn't changed
    assert_eq!(memory.len().unwrap(), 5);
}

#[test]
fn test_free_negative_address() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Test with negative index
    assert!(matches!(
        memory.free(-1),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Verify memory state hasn't changed
    assert_eq!(memory.len().unwrap(), 5);
}

#[test]
fn test_free_multiple_times() {
    let mut memory = Memory::new();
    memory.alloc(10).unwrap();

    // Free from index 8
    memory.free(8).unwrap();
    assert_eq!(memory.len().unwrap(), 8);

    // Free from index 5
    memory.free(5).unwrap();
    assert_eq!(memory.len().unwrap(), 5);

    // Free from index 0
    memory.free(0).unwrap();
    assert!(memory.is_empty());
}

#[test]
fn test_free_then_allocate() {
    let mut memory = Memory::new();
    memory.alloc(10).unwrap();

    // Free half the memory
    memory.free(5).unwrap();
    assert_eq!(memory.len().unwrap(), 5);

    // Allocate new memory
    memory.alloc(3).unwrap();
    assert_eq!(memory.len().unwrap(), 8);

    // Verify old values are intact
    for i in 0..5 {
        assert_eq!(memory.load(i).unwrap(), 0);
    }
}

#[test]
fn test_free_capacity_reduction() {
    let mut memory = Memory::new();
    memory.alloc(1000).unwrap();

    // Free most of the memory
    let index_to_keep = 100;
    memory.free(index_to_keep).unwrap();

    // Verify capacity has been reduced
    assert_eq!(memory.0.capacity(), index_to_keep as usize);
}

#[test]
fn test_store_range_empty_memory() {
    let mut memory = Memory::new();
    let values = vec![1, 2, 3];

    // Trying to store to empty memory should fail
    assert!(matches!(
        memory.store_range(0, &values),
        Err(TemporaryError::IndexOutOfBounds)
    ));
}

#[test]
fn test_store_range_sanity() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();
    let values = vec![10, 20, 30];

    // Store range at beginning
    memory.store_range(0, &values).unwrap();

    // Verify values were stored correctly
    assert_eq!(memory.load(0).unwrap(), 10);
    assert_eq!(memory.load(1).unwrap(), 20);
    assert_eq!(memory.load(2).unwrap(), 30);

    // Verify remaining memory is unchanged
    assert_eq!(memory.load(3).unwrap(), 0);
    assert_eq!(memory.load(4).unwrap(), 0);
}

#[test]
fn test_store_range_at_offset() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();
    let values = vec![10, 20];

    // Store range at offset 2
    memory.store_range(2, &values).unwrap();

    // Verify values were stored correctly
    assert_eq!(memory.load(0).unwrap(), 0);
    assert_eq!(memory.load(1).unwrap(), 0);
    assert_eq!(memory.load(2).unwrap(), 10);
    assert_eq!(memory.load(3).unwrap(), 20);
    assert_eq!(memory.load(4).unwrap(), 0);
}

#[test]
fn test_store_range_exact_fit() {
    let mut memory = Memory::new();
    memory.alloc(3).unwrap();
    let values = vec![1, 2, 3];

    // Store range that exactly fits memory
    memory.store_range(0, &values).unwrap();

    // Verify all values were stored
    for i in 0..3 {
        assert_eq!(memory.load(i as Word).unwrap(), (i + 1) as Word);
    }
}

#[test]
fn test_store_range_overflow() {
    let mut memory = Memory::new();
    memory.alloc(3).unwrap();
    let values = vec![1, 2, 3, 4]; // One more than allocated

    // Try to store more values than available space
    assert!(matches!(
        memory.store_range(0, &values),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Verify memory wasn't modified
    assert_eq!(memory.load(0).unwrap(), 0);
}

#[test]
fn test_store_range_invalid_start_address() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();
    let values = vec![1, 2];

    // Try to store at invalid address
    assert!(matches!(
        memory.store_range(4, &values), // Would overflow
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Try to store at out of bounds address
    assert!(matches!(
        memory.store_range(5, &values),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Try to store at very large address
    assert!(matches!(
        memory.store_range(Word::MAX, &values),
        Err(TemporaryError::IndexOutOfBounds)
    ));
}

#[test]
fn test_store_range_negative_address() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();
    let values = vec![1, 2];

    // Try to store at negative address
    assert!(matches!(
        memory.store_range(-1, &values),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Verify memory wasn't modified
    assert_eq!(memory.load(0).unwrap(), 0);
}

#[test]
fn test_store_range_empty_slice() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();
    let values: Vec<Word> = vec![];

    // Store empty slice
    memory.store_range(0, &values).unwrap();

    // Verify memory wasn't modified
    assert_eq!(memory.load(0).unwrap(), 0);
}

#[test]
fn test_store_range_multiple_times() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // First store
    let values1 = vec![1, 2];
    memory.store_range(0, &values1).unwrap();

    // Second store overlapping
    let values2 = vec![3, 4];
    memory.store_range(1, &values2).unwrap();

    // Verify final state
    assert_eq!(memory.load(0).unwrap(), 1);
    assert_eq!(memory.load(1).unwrap(), 3);
    assert_eq!(memory.load(2).unwrap(), 4);
    assert_eq!(memory.load(3).unwrap(), 0);
    assert_eq!(memory.load(4).unwrap(), 0);
}

#[test]
fn test_store_range_max_values() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();
    let values = vec![Word::MAX, Word::MIN, Word::MAX];

    // Store range with extreme values
    memory.store_range(1, &values).unwrap();

    // Verify values were stored correctly
    assert_eq!(memory.load(1).unwrap(), Word::MAX);
    assert_eq!(memory.load(2).unwrap(), Word::MIN);
    assert_eq!(memory.load(3).unwrap(), Word::MAX);
}

#[test]
fn test_store_range_after_free() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Free part of memory
    memory.free(3).unwrap();

    let values = vec![1, 2];

    // Try to store in freed space
    assert!(matches!(
        memory.store_range(2, &values),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Store in remaining space
    memory.store_range(0, &values).unwrap();

    // Verify values were stored correctly
    assert_eq!(memory.load(0).unwrap(), 1);
    assert_eq!(memory.load(1).unwrap(), 2);
}

#[test]
fn test_load_range_empty_memory() {
    let mut memory = Memory::new();

    // Trying to load from empty memory should fail
    assert!(matches!(
        memory.load_range(0, 1),
        Err(TemporaryError::IndexOutOfBounds)
    ));
}

#[test]
fn test_load_range_sanity() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Store some test values
    let test_values = vec![10, 20, 30];
    memory.store_range(0, &test_values).unwrap();

    // Load range from beginning
    let loaded = memory.load_range(0, 3).unwrap();

    // Verify loaded values
    assert_eq!(loaded, test_values);
}

#[test]
fn test_load_range_at_offset() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Store test values
    memory.store_range(0, &[1, 2, 3, 4, 5]).unwrap();

    // Load range from offset
    let loaded = memory.load_range(2, 2).unwrap();

    // Verify loaded values
    assert_eq!(loaded, vec![3, 4]);
}

#[test]
fn test_load_range_exact_size() {
    let mut memory = Memory::new();
    memory.alloc(3).unwrap();

    // Store test values
    memory.store_range(0, &[1, 2, 3]).unwrap();

    // Load entire memory range
    let loaded = memory.load_range(0, 3).unwrap();

    // Verify loaded values
    assert_eq!(loaded, vec![1, 2, 3]);
}

#[test]
fn test_load_range_overflow() {
    let mut memory = Memory::new();
    memory.alloc(3).unwrap();

    // Try to load more values than available
    assert!(matches!(
        memory.load_range(0, 4),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Try to load with address + size overflow
    assert!(matches!(
        memory.load_range(2, 2),
        Err(TemporaryError::IndexOutOfBounds)
    ));
}

#[test]
fn test_load_range_invalid_start_address() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Try to load from out of bounds address
    assert!(matches!(
        memory.load_range(5, 1),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Try to load from very large address
    assert!(matches!(
        memory.load_range(Word::MAX, 1),
        Err(TemporaryError::IndexOutOfBounds)
    ));
}

#[test]
fn test_load_range_negative_address() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Try to load from negative address
    assert!(matches!(
        memory.load_range(-1, 1),
        Err(TemporaryError::IndexOutOfBounds)
    ));
}

#[test]
fn test_load_range_zero_size() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Load range of size 0
    let loaded = memory.load_range(0, 0).unwrap();

    // Verify empty result
    assert!(loaded.is_empty());
}

#[test]
fn test_load_range_negative_size() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Try to load with negative size
    assert!(matches!(
        memory.load_range(0, -1),
        Err(TemporaryError::Overflow)
    ));
}

#[test]
fn test_load_range_maximum_size() {
    let mut memory = Memory::new();
    let size = 100; // Choose a reasonably large size
    memory.alloc(size).unwrap();

    // Store some test values
    for i in 0..size {
        memory.store(i as Word, i as Word).unwrap();
    }

    // Load entire range
    let loaded = memory.load_range(0, size as Word).unwrap();

    // Verify all values
    for (i, &value) in loaded.iter().enumerate() {
        assert_eq!(value, i as Word);
    }
}

#[test]
fn test_load_range_after_modification() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Initial store
    memory.store_range(0, &[1, 2, 3, 4, 5]).unwrap();

    // Modify some values
    memory.store(2, 30).unwrap();
    memory.store(3, 40).unwrap();

    // Load modified range
    let loaded = memory.load_range(1, 3).unwrap();

    // Verify loaded values reflect modifications
    assert_eq!(loaded, vec![2, 30, 40]);
}

#[test]
fn test_load_range_after_free() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Store initial values
    memory.store_range(0, &[1, 2, 3, 4, 5]).unwrap();

    // Free part of memory
    memory.free(3).unwrap();

    // Try to load from freed space
    assert!(matches!(
        memory.load_range(2, 2),
        Err(TemporaryError::IndexOutOfBounds)
    ));

    // Load from remaining space
    let loaded = memory.load_range(0, 2).unwrap();
    assert_eq!(loaded, vec![1, 2]);
}

#[test]
fn test_load_range_large_size_overflow() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Try to load with size that would cause overflow when added to address
    assert!(matches!(
        memory.load_range(Word::MAX - 1, 2),
        Err(TemporaryError::IndexOutOfBounds)
    ));
}

#[test]
fn test_load_range_consecutive_loads() {
    let mut memory = Memory::new();
    memory.alloc(5).unwrap();

    // Store test values
    memory.store_range(0, &[1, 2, 3, 4, 5]).unwrap();

    // Perform consecutive loads
    let first = memory.load_range(0, 2).unwrap();
    let second = memory.load_range(2, 2).unwrap();
    let third = memory.load_range(4, 1).unwrap();

    // Verify all loads
    assert_eq!(first, vec![1, 2]);
    assert_eq!(second, vec![3, 4]);
    assert_eq!(third, vec![5]);
}
