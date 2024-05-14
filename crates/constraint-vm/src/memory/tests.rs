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
