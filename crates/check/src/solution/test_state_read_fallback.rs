use super::*;

type ContentAddr = u8;
type K = [Word];
type V = [Word];
struct PreState {
    state: HashMap<ContentAddress, HashMap<Key, Value>>,
}

fn s(post: &[(ContentAddr, &K, &V)], pre: &[(ContentAddr, &K, &V)]) -> (PostState, PreState) {
    let mut pre_state = HashMap::new();
    for (addr, key, value) in pre.iter() {
        pre_state
            .entry(ContentAddress([*addr; 32]))
            .or_insert_with(HashMap::new)
            .insert(key.to_vec(), value.to_vec());
    }
    let mut post_state = HashMap::new();
    for (addr, key, value) in post.iter() {
        post_state
            .entry(ContentAddress([*addr; 32]))
            .or_insert_with(HashMap::new)
            .insert(key.to_vec(), value.to_vec());
    }
    (
        PostState { state: post_state },
        PreState { state: pre_state },
    )
}

impl StateRead for PreState {
    type Error = String;

    fn key_range(
        &self,
        contract_addr: ContentAddress,
        mut key: Key,
        num_values: usize,
    ) -> Result<Vec<Vec<Word>>, Self::Error> {
        let mut result = vec![];
        if let Some(content) = self.state.get(&contract_addr) {
            for _ in 0..num_values {
                match content.get(&key) {
                    Some(value) => {
                        result.push(value.clone());
                    }
                    None => result.push(vec![]),
                }
                match next_key(key) {
                    Some(next_key) => key = next_key,
                    None => break,
                }
            }
        }
        Ok(result)
    }
}

fn c(i: ContentAddr) -> ContentAddress {
    ContentAddress([i; 32])
}

#[test]
fn test_fallback() {
    let (post, pre) = s(&[], &[(0, &[1], &[42]), (0, &[2], &[43])]);
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![1], 1).unwrap(),
        vec![vec![42]]
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![1], 2).unwrap(),
        vec![vec![42], vec![43]]
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![0], 2).unwrap(),
        vec![vec![], vec![42]]
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![2], 2).unwrap(),
        vec![vec![43], vec![]]
    );

    let (post, pre) = s(
        &[(0, &[1], &[42]), (0, &[2], &[43])],
        &[(0, &[1], &[52]), (0, &[2], &[53])],
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![1], 1).unwrap(),
        vec![vec![42]]
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![1], 2).unwrap(),
        vec![vec![42], vec![43]]
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![0], 2).unwrap(),
        vec![vec![], vec![42]]
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![2], 2).unwrap(),
        vec![vec![43], vec![]]
    );

    let (post, pre) = s(
        &[(0, &[1], &[42]), (0, &[3], &[43])],
        &[(0, &[1], &[52]), (0, &[2], &[53])],
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![1], 1).unwrap(),
        vec![vec![42]]
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![1], 2).unwrap(),
        vec![vec![42], vec![53]]
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![0], 2).unwrap(),
        vec![vec![], vec![42]]
    );
    assert_eq!(
        read_or_fallback(&post, &pre, c(0), vec![2], 2).unwrap(),
        vec![vec![53], vec![43]]
    );
}
