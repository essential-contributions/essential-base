use std::collections::HashMap;

use essential_types::convert::word_4_from_u8_32;

use super::*;

#[test]
fn test_pop_memory_address() {
    let mut stack = Stack::default();
    stack.push(10).unwrap();
    let addr = pop_memory_address(&mut stack).unwrap();
    assert_eq!(addr, 10);

    stack.push(-10).unwrap();
    pop_memory_address(&mut stack).unwrap_err();
}

#[test]
fn test_pop_key_range_args() {
    let mut stack = Stack::default();
    stack.extend(vec![1, 2, 3, 4, 4, 10]).unwrap();
    let (key, num_keys) = pop_key_range_args(&mut stack).unwrap();
    assert_eq!(key, vec![1, 2, 3, 4]);
    assert_eq!(num_keys, 10);

    stack.extend(vec![1, 2, 3, 4, 5, 10]).unwrap();
    pop_key_range_args(&mut stack).unwrap_err();

    stack.extend(vec![1, 2, 3, 4, -4, 10]).unwrap();
    pop_key_range_args(&mut stack).unwrap_err();

    stack.extend(vec![1, 2, 3, 4, 4, -10]).unwrap();
    pop_key_range_args(&mut stack).unwrap_err();
}

#[test]
fn test_write_values_to_memory() {
    let mut memory = Memory::default();
    write_values_to_memory(0, vec![], &mut memory).unwrap();
    let expected: &[i64] = &[];
    assert_eq!(memory.get(..).unwrap(), expected);

    let mut memory = Memory::default();
    memory.alloc(4 + 3).unwrap();
    write_values_to_memory(0, vec![vec![42, 43], vec![44]], &mut memory).unwrap();
    let expected: &[i64] = &[4, 2, 6, 1, 42, 43, 44];
    assert_eq!(memory.get(..).unwrap(), expected);

    let mut memory = Memory::default();
    memory.alloc(4 + 3).unwrap();
    write_values_to_memory(1, vec![vec![42, 43], vec![44]], &mut memory).unwrap_err();

    let mut memory = Memory::default();
    write_values_to_memory(0, vec![vec![42, 43], vec![44]], &mut memory).unwrap_err();
}

#[test]
fn test_read_key_range_sync() {
    let mut state = State::default();
    let mut stack = Stack::default();
    let mut memory = Memory::default();
    let contract_addr = ContentAddress([1; 32]);

    stack.extend([42, 43, 2, 2, 0]).unwrap();
    key_range_sync(&state, &contract_addr, &mut stack, &mut memory).unwrap();
    let expected: &[i64] = &[];
    assert_eq!(memory.get(..).unwrap(), expected);

    memory.alloc(2 + 2).unwrap();
    state.contracts.insert(
        contract_addr.clone(),
        [(vec![42, 43], vec![1, 2])].into_iter().collect(),
    );

    stack.extend([42, 43, 2, 2, 0]).unwrap();
    key_range_sync(&state, &contract_addr, &mut stack, &mut memory).unwrap();
    let expected: &[i64] = &[2, 2, 1, 2];
    assert_eq!(memory.get(..).unwrap(), expected);
}

#[test]
fn test_read_key_range_ext_sync() {
    let mut state = State::default();
    let mut stack = Stack::default();
    let mut memory = Memory::default();
    let contract_addr = ContentAddress([1; 32]);
    let contract_words = word_4_from_u8_32(contract_addr.0);

    stack.extend(contract_words).unwrap();
    stack.extend([42, 43, 2, 2, 0]).unwrap();
    key_range_ext_sync(&state, &mut stack, &mut memory).unwrap();
    let expected: &[i64] = &[];
    assert_eq!(memory.get(..).unwrap(), expected);

    memory.alloc(2 + 2).unwrap();
    state.contracts.insert(
        contract_addr.clone(),
        [(vec![42, 43], vec![1, 2])].into_iter().collect(),
    );

    stack.extend(contract_words).unwrap();
    stack.extend([42, 43, 2, 2, 0]).unwrap();
    key_range_ext_sync(&state, &mut stack, &mut memory).unwrap();
    let expected: &[i64] = &[2, 2, 1, 2];
    assert_eq!(memory.get(..).unwrap(), expected);
}

#[derive(Default)]
struct State {
    contracts: HashMap<ContentAddress, HashMap<Key, Value>>,
}

impl StateReadSync for State {
    type Error = String;

    fn key_range(
        &self,
        contract_addr: ContentAddress,
        mut key: Key,
        num_keys: usize,
    ) -> Result<Vec<Value>, Self::Error> {
        let Some(contracts) = self.contracts.get(&contract_addr) else {
            return Ok(Vec::new());
        };
        let mut values = Vec::new();
        for _ in 0..num_keys {
            let Some(value) = contracts.get(&key).cloned() else {
                return Ok(values);
            };
            values.push(value);
            let Some(k) = next_key(key) else {
                return Ok(values);
            };
            key = k;
        }
        Ok(values)
    }
}
fn next_key(mut key: Key) -> Option<Key> {
    for w in key.iter_mut().rev() {
        match *w {
            Word::MAX => *w = Word::MIN,
            _ => {
                *w += 1;
                return Some(key);
            }
        }
    }
    None
}
