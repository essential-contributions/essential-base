use super::*;
use essential_types::{ContentAddress, PredicateAddress};

#[test]
fn test_predicate_exists() {
    // Sanity
    let (mut stack, data, cache) = setup(
        &[Setup {
            contract_addr: [0; 32],
            predicate_addr: [0; 32],
            args: vec![vec![1, 2, 3, 4]],
        }],
        0,
    );
    assert!(check(&mut stack, Arc::new(data), &cache).unwrap());

    // Multiple
    let (mut stack, data, cache) = setup(
        &[
            Setup {
                contract_addr: [0; 32],
                predicate_addr: [0; 32],
                args: vec![vec![1, 2, 3, 4]],
            },
            Setup {
                contract_addr: [1; 32],
                predicate_addr: [1; 32],
                args: vec![vec![1, 3, 4], vec![5]],
            },
            Setup {
                contract_addr: [5; 32],
                predicate_addr: [6; 32],
                args: vec![vec![]],
            },
        ],
        1,
    );
    assert!(check(&mut stack, Arc::new(data), &cache).unwrap());

    // Duplicate
    let (mut stack, data, cache) = setup(
        &[
            Setup {
                contract_addr: [0; 32],
                predicate_addr: [0; 32],
                args: vec![vec![1, 2, 3, 4]],
            },
            Setup {
                contract_addr: [0; 32],
                predicate_addr: [0; 32],
                args: vec![vec![1, 2, 3, 4]],
            },
            Setup {
                contract_addr: [5; 32],
                predicate_addr: [6; 32],
                args: vec![vec![]],
            },
        ],
        1,
    );
    assert!(check(&mut stack, Arc::new(data), &cache).unwrap());

    // Not exists
    let (mut stack, mut data, cache) = setup(
        &[
            Setup {
                contract_addr: [0; 32],
                predicate_addr: [0; 32],
                args: vec![vec![1, 2, 3, 4]],
            },
            Setup {
                contract_addr: [5; 32],
                predicate_addr: [6; 32],
                args: vec![vec![]],
            },
        ],
        1,
    );
    data[1].predicate_to_solve = PredicateAddress {
        contract: ContentAddress([0; 32]),
        predicate: ContentAddress([0; 32]),
    };
    assert!(!check(&mut stack, Arc::new(data), &cache).unwrap());

    // Not exists
    let (mut stack, data, cache) = setup(
        &[
            Setup {
                contract_addr: [0; 32],
                predicate_addr: [0; 32],
                args: vec![vec![1, 2, 3, 4]],
            },
            Setup {
                contract_addr: [5; 32],
                predicate_addr: [6; 32],
                args: vec![vec![]],
            },
        ],
        1,
    );
    stack.pop().unwrap();
    check(&mut stack, Arc::new(data), &cache).unwrap_err();
}

fn check(stack: &mut Stack, data: Arc<Vec<Solution>>, cache: &LazyCache) -> OpResult<bool> {
    predicate_exists(stack, data, cache)?;
    let s = stack.iter().cloned().collect::<Vec<_>>();
    assert_eq!(s.len(), 1);
    let s: bool = s[0] == 1;
    Ok(s)
}

struct Setup {
    contract_addr: [u8; 32],
    predicate_addr: [u8; 32],
    args: Vec<Vec<Word>>,
}

fn setup(input: &[Setup], i: usize) -> (Stack, Vec<Solution>, LazyCache) {
    let mut stack = Stack::default();
    let cache = LazyCache::default();
    let data: Vec<_> = input
        .iter()
        .map(|s| Solution {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress(s.contract_addr),
                predicate: ContentAddress(s.predicate_addr),
            },
            predicate_data: s.args.clone(),
            state_mutations: Default::default(),
        })
        .collect();
    let words: Vec<_> = data
        .iter()
        .map(|d| {
            let words = d.predicate_data.iter().flat_map(|slot| {
                Some(slot.len() as Word)
                    .into_iter()
                    .chain(slot.iter().cloned())
            });
            let words = words.chain(word_4_from_u8_32(d.predicate_to_solve.contract.0));
            let words = words.chain(word_4_from_u8_32(d.predicate_to_solve.predicate.0));
            let bytes: Vec<_> = words.flat_map(bytes_from_word).collect();
            word_4_from_u8_32(sha256(&bytes))
        })
        .collect();
    stack.extend(words[i]).unwrap();
    (stack, data, cache)
}
