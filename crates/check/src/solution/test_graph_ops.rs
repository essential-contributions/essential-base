use essential_types::predicate::{Edge, Node};

use super::*;

/// 0   1
///  \ /
///   2
///  / \
/// 3   4
///  \ /
///   5
fn p() -> Predicate {
    Predicate {
        nodes: vec![
            Node {
                edge_start: 0,
                program_address: ContentAddress([0; 32]),
            },
            Node {
                edge_start: 1,
                program_address: ContentAddress([1; 32]),
            },
            Node {
                edge_start: 2,
                program_address: ContentAddress([2; 32]),
            },
            Node {
                edge_start: 4,
                program_address: ContentAddress([3; 32]),
            },
            Node {
                edge_start: 5,
                program_address: ContentAddress([4; 32]),
            },
            Node {
                edge_start: Edge::MAX,
                program_address: ContentAddress([5; 32]),
            },
        ],
        edges: vec![2, 2, 3, 4, 5, 5],
    }
}

///   0
///  / \
/// 1   2
/// |\  |
/// 3 4 5
///    \|
///     6
fn p3() -> Predicate {
    Predicate {
        nodes: vec![
            Node {
                edge_start: 0,
                program_address: ContentAddress([0; 32]), // 0
            },
            Node {
                edge_start: 2,
                program_address: ContentAddress([1; 32]), // 1
            },
            Node {
                edge_start: 4,
                program_address: ContentAddress([2; 32]), // 2
            },
            Node {
                edge_start: 5,
                program_address: ContentAddress([4; 32]), // 3
            },
            Node {
                edge_start: 6,
                program_address: ContentAddress([5; 32]), // 4
            },
            Node {
                edge_start: Edge::MAX,
                program_address: ContentAddress([3; 32]), // 5
            },
            Node {
                edge_start: Edge::MAX,
                program_address: ContentAddress([6; 32]), // 6
            },
        ],
        edges: vec![1, 2, 5, 3, 4, 6, 6],
    }
}

/// Topological sort of a simple graph.
/// Except parallel nodes are in their own inner list
/// 0   1
///  \ /
///   2
///  / \
/// 3   4
///  \ /
///   5
///
/// [[0, 1], [2], [3, 4], [5]]
#[test]
fn test_parallel_top_sort() {
    let predicate = p();
    let parent_map = create_parent_map::<String>(&predicate).unwrap();

    assert_eq!(
        vec![vec![0, 1], vec![2], vec![3, 4], vec![5],],
        parallel_topo_sort::<String>(&predicate, &parent_map).unwrap()
    );
}

/// 0   1
///  \ /
///   2
///  / \
/// 3   4
///  \ /
///   5
///
/// {0: [], 1: [], 2: [0, 1], 3: [2], 4: [2], 5: [3, 4]}
#[test]
fn test_create_parent_map() {
    let predicate = p();

    let parent_map = create_parent_map::<String>(&predicate).unwrap();

    let expected: BTreeMap<_, _> = [
        (0, vec![]),
        (1, vec![]),
        (2, vec![0, 1]),
        (3, vec![2]),
        (4, vec![2]),
        (5, vec![3, 4]),
    ]
    .into_iter()
    .collect();

    assert_eq!(expected, parent_map);

    //   0[0]
    //  /  \
    // 1[1] 2[2]
    // |   \    \
    // 3[5] 4[3] 5[4]
    //       \   |
    //        6[6]
    let predicate = p3();
    let parent_map = create_parent_map::<String>(&predicate).unwrap();

    let expected: BTreeMap<_, _> = [
        (0, vec![]),
        (1, vec![0]),
        (2, vec![0]),
        (3, vec![1]),
        (4, vec![2]),
        (5, vec![1]),
        (6, vec![3, 4]),
    ]
    .into_iter()
    .collect();

    assert_eq!(expected, parent_map);
}

/// 0   1
///  \ /
///   2
///  / \
/// 3   4
///  \ /
///   5
///
/// {0: 0, 1: 0, 2: 2, 3: 1, 4: 1, 5: 2}
#[test]
fn test_in_degrees() {
    let predicate = p();

    let parent_map = create_parent_map::<String>(&predicate).unwrap();

    let in_degrees = in_degrees(predicate.nodes.len(), &parent_map);

    let expected: BTreeMap<_, _> = [(0, 0), (1, 0), (2, 2), (3, 1), (4, 1), (5, 2)]
        .into_iter()
        .collect();

    assert_eq!(expected, in_degrees);
}

/// 0   1
///  \ /
///   2
///  / \
/// 3   4
///  \ /
///   5
///
/// {0: 0, 1: 0, 2: 2, 3: 1, 4: 1, 5: 2}
/// {0: 0, 1: 0, 2: 1, 3: 1, 4: 1, 5: 2}
#[test]
fn test_reduce_in_degrees() {
    let predicate = p();

    let parent_map = create_parent_map::<String>(&predicate).unwrap();

    let mut in_degrees = in_degrees(predicate.nodes.len(), &parent_map);

    reduce_in_degrees(&mut in_degrees, predicate.node_edges(0).unwrap());

    let expected: BTreeMap<_, _> = [(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 2)]
        .into_iter()
        .collect();

    assert_eq!(expected, in_degrees);
}

/// 0   1
///  \ /
///   2
///  / \
/// 3   4
///  \ /
///   5
///
/// [0, 1]
#[test]
fn test_find_nodes_with_no_parents() {
    let predicate = p();

    let parent_map = create_parent_map::<String>(&predicate).unwrap();

    let in_degrees = in_degrees(predicate.nodes.len(), &parent_map);

    let nodes = find_nodes_with_no_parents(&in_degrees);

    assert_eq!(vec![0, 1], nodes);
}

/// 0   1
///  \ /
///   2
///  / \
/// 3   4
/// |   |
/// 5   6
///  \ /
///   7
fn p2() -> Predicate {
    Predicate {
        nodes: vec![
            Node {
                edge_start: 0,
                program_address: ContentAddress([0; 32]),
            },
            Node {
                edge_start: 1,
                program_address: ContentAddress([1; 32]),
            },
            Node {
                edge_start: 2,
                program_address: ContentAddress([2; 32]),
            },
            Node {
                edge_start: 4,
                program_address: ContentAddress([3; 32]),
            },
            Node {
                edge_start: 5,
                program_address: ContentAddress([4; 32]),
            },
            Node {
                edge_start: 6,
                program_address: ContentAddress([5; 32]),
            },
            Node {
                edge_start: 7,
                program_address: ContentAddress([6; 32]),
            },
            Node {
                edge_start: Edge::MAX,
                program_address: ContentAddress([7; 32]),
            },
        ],
        edges: vec![2, 2, 3, 4, 5, 6, 7, 7],
    }
}

/// 0   1
///  \ /
///   2
///  / \
/// 3  *4
/// |   |
/// 5   6
///  \ /
///   7
///
/// {4, 6, 7}
#[test]
fn test_find_deferred() {
    let predicate = p2();

    let f = |node: &Node| -> bool { node.program_address.0[0] == 4 };
    let deferred = find_deferred(&predicate, f);

    let expected: HashSet<_> = [4, 6, 7].into_iter().collect();
    assert_eq!(expected, deferred);
}

/// 0   1
///  \ /
///   2
///  / \
/// 3  *4
/// |   |
/// 5   6
///  \ /
///   7
///
/// [0: false, 1: false, 2: true, 3: false, 4: false, 5: true, 6: false, 7: false]
#[test]
fn test_should_cache() {
    let predicate = p2();
    let deferred: HashSet<_> = [4, 6, 7].into_iter().collect();

    let mut results = HashMap::new();
    for node in 0..predicate.nodes.len() {
        let r = should_cache(node as u16, &predicate, &deferred);
        results.insert(node as u16, r);
    }

    let expected: HashMap<_, _> = [
        (0, false),
        (1, false),
        (2, true),
        (3, false),
        (4, false),
        (5, true),
        (6, false),
        (7, false),
    ]
    .into_iter()
    .collect();

    assert_eq!(expected, results);
}
