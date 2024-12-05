use super::*;

#[test]
fn test_encode_predicate() {
    let predicate = Predicate {
        nodes: vec![
            Node {
                edge_start: 0,
                program_address: ContentAddress([0; 32]),
                reads: Reads::default(),
            },
            Node {
                edge_start: u16::MAX,
                program_address: ContentAddress([1; 32]),
                reads: Reads::default(),
            },
            Node {
                edge_start: 2,
                program_address: ContentAddress([2; 32]),
                reads: Reads::Post,
            },
            Node {
                edge_start: u16::MAX,
                program_address: ContentAddress([3; 32]),
                reads: Reads::default(),
            },
            Node {
                edge_start: u16::MAX,
                program_address: ContentAddress([4; 32]),
                reads: Reads::default(),
            },
        ],
        edges: vec![1, 2, 3, 4],
    };
    let encoded: Vec<u8> = encode_predicate(&predicate).unwrap().collect();
    let expected = [
        5u16.to_be_bytes().to_vec(), // len of nodes
        // node 0
        0u16.to_be_bytes().to_vec(), // edge_start
        vec![0; 32],                 // program_address
        vec![
            0u8, // reads
        ],
        // node 1
        u16::MAX.to_be_bytes().to_vec(), // edge_start
        vec![1; 32],                     // program_address
        vec![
            0, // reads
        ],
        // node 2
        2u16.to_be_bytes().to_vec(), // edge_start
        vec![2; 32],                 // program_address
        vec![
            1, // reads
        ],
        // node 3
        u16::MAX.to_be_bytes().to_vec(), // edge_start
        vec![3; 32],                     // program_address
        vec![
            0, // reads
        ],
        // node 4
        u16::MAX.to_be_bytes().to_vec(), // edge_start
        vec![4; 32],                     // program_address
        vec![
            0, // reads
        ],
        4u16.to_be_bytes().to_vec(), // len of edges
        [1u16, 2, 3, 4]
            .into_iter()
            .flat_map(|x| x.to_be_bytes())
            .collect::<Vec<u8>>(),
    ]
    .concat();
    assert_eq!(encoded, expected);
    let decoded = decode_predicate(&encoded).unwrap();
    assert_eq!(decoded, predicate);
}
