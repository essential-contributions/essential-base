use essential_check::{solution, vm::asm};
use essential_hash::content_addr;
use essential_types::{
    contract::Contract,
    predicate::{Edge, Node, Predicate, Program, Reads},
    solution::{Mutation, Solution, SolutionData},
    ContentAddress, PredicateAddress, Word,
};
use std::{collections::HashMap, sync::Arc};
use util::{empty_solution, State};

pub mod util;

fn test_predicate_addr() -> PredicateAddress {
    PredicateAddress {
        contract: ContentAddress([0; 32]),
        predicate: ContentAddress([0; 32]),
    }
}

fn test_solution_data() -> SolutionData {
    SolutionData {
        predicate_to_solve: test_predicate_addr(),
        decision_variables: vec![],
        state_mutations: vec![],
    }
}

fn test_mutation(salt: usize) -> Mutation {
    Mutation {
        key: vec![salt as Word; 4],
        value: vec![42],
    }
}

#[test]
fn solution_data_mut_not_be_empty() {
    let solution = empty_solution();
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::Data(solution::InvalidSolutionData::Empty),
    ));
}

#[test]
fn too_many_solution_data() {
    let solution = Solution {
        data: (0..solution::MAX_SOLUTION_DATA + 1)
            .map(|_| test_solution_data())
            .collect(),
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::Data(solution::InvalidSolutionData::TooMany(n))
            if n == solution::MAX_SOLUTION_DATA + 1
    ));
}

#[test]
fn too_many_decision_variables() {
    let solution = Solution {
        data: vec![SolutionData {
            predicate_to_solve: test_predicate_addr(),
            decision_variables: vec![vec![0]; (solution::MAX_DECISION_VARIABLES + 1) as usize],
            state_mutations: vec![],
        }],
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::Data(solution::InvalidSolutionData::TooManyDecisionVariables(0, n))
            if n == solution::MAX_DECISION_VARIABLES as usize + 1
    ));
}

#[test]
fn too_many_state_mutations() {
    let solution = Solution {
        data: vec![SolutionData {
            predicate_to_solve: test_predicate_addr(),
            decision_variables: vec![],
            state_mutations: (0..(solution::MAX_STATE_MUTATIONS + 1))
                .map(test_mutation)
                .collect(),
        }],
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::StateMutations(solution::InvalidStateMutations::TooMany(n))
            if n == solution::MAX_STATE_MUTATIONS + 1
    ));
}

#[test]
fn multiple_mutations_for_slot() {
    let solution = Solution {
        data: vec![SolutionData {
            predicate_to_solve: test_predicate_addr(),
            decision_variables: vec![],
            state_mutations: vec![
                Mutation {
                    key: vec![0; 4],
                    value: vec![42],
                };
                2
            ],
        }],
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::StateMutations(solution::InvalidStateMutations::MultipleMutationsForSlot(addr, key))
            if addr == test_predicate_addr() && key == [0; 4]
    ));
}

// A simple test to check that resulting stacks are passed from parents to children.
//
// ```ignore
// a     b
//  \   /
//   \ /
//    v
//    c
// ```
#[tokio::test]
async fn predicate_graph_stack_passing() {
    use essential_vm::asm::short::*;
    let _ = tracing_subscriber::fmt::try_init();
    let a = Program(asm::to_bytes([PUSH(1), PUSH(2), PUSH(3), HLT]).collect());
    let b = Program(asm::to_bytes([PUSH(4), PUSH(5), PUSH(6), HLT]).collect());
    let c = Program(
        asm::to_bytes([
            // Stack should already have `[1, 2, 3, 4, 5, 6]`.
            PUSH(1),
            PUSH(2),
            PUSH(3),
            PUSH(4),
            PUSH(5),
            PUSH(6),
            // a `len` for `EqRange`.
            PUSH(6), // EqRange len
            EQRA,
            HLT,
        ])
        .collect(),
    );

    let a_ca = content_addr(&a);
    let b_ca = content_addr(&b);
    let c_ca = content_addr(&c);

    let node = |program_address, edge_start| Node {
        program_address,
        edge_start,
        reads: Reads::Pre, // unused for this test.
    };
    let nodes = vec![
        node(a_ca.clone(), 0),
        node(b_ca.clone(), 1),
        node(c_ca.clone(), Edge::MAX),
    ];
    let edges = vec![2, 2];
    let predicate = Predicate { nodes, edges };
    let contract = Contract::without_salt(vec![predicate]);
    let pred_addr = PredicateAddress {
        contract: content_addr(&contract),
        predicate: content_addr(&contract.predicates[0]),
    };

    // Create a solution that "solves" our predicate.
    let solution = Solution {
        data: vec![SolutionData {
            predicate_to_solve: pred_addr.clone(),
            decision_variables: Default::default(),
            state_mutations: vec![],
        }],
    };

    // First, validate both predicates and solution.
    essential_check::predicate::check(&contract.predicates[0]).unwrap();
    essential_check::solution::check(&solution).unwrap();

    // There's only one predicate to solve.
    let predicate = Arc::new(contract.predicates[0].clone());
    let get_predicate = |addr: &PredicateAddress| {
        assert_eq!(&pred_addr, addr);
        predicate.clone()
    };
    let programs: HashMap<ContentAddress, Arc<Program>> = vec![
        (a_ca, Arc::new(a)),
        (b_ca, Arc::new(b)),
        (c_ca, Arc::new(c)),
    ]
    .into_iter()
    .collect();
    let get_program: Arc<HashMap<_, _>> = Arc::new(programs);

    // Run the check, and ensure ok and gas aren't 0.
    let gas = solution::check_predicates(
        &State::EMPTY,
        &State::EMPTY,
        Arc::new(solution),
        get_predicate,
        get_program,
        Arc::new(solution::CheckPredicateConfig::default()),
    )
    .await
    .unwrap();

    assert!(gas > 0);
}

// A simple test to check that resulting memories are passed from parents to children.
//
// ```ignore
// a     b
//  \   /
//   \ /
//    v
//    c
// ```
#[tokio::test]
async fn predicate_graph_memory_passing() {
    use essential_vm::asm::short::*;
    let _ = tracing_subscriber::fmt::try_init();
    // Store `[1, 2, 3]` at the start of memory.
    let a = Program(
        asm::to_bytes([
            PUSH(3),
            ALOC,
            PUSH(1),
            STO,
            PUSH(1),
            PUSH(2),
            STO,
            PUSH(2),
            PUSH(3),
            STO,
            HLT,
        ])
        .collect(),
    );
    // Store `[4, 5, 6]` at the start of memory.
    let b = Program(
        asm::to_bytes([
            PUSH(3),
            ALOC,
            PUSH(4),
            STO,
            PUSH(1),
            PUSH(5),
            STO,
            PUSH(2),
            PUSH(6),
            STO,
            HLT,
        ])
        .collect(),
    );
    let c = Program(
        asm::to_bytes([
            // Memory should already have `[1, 2, 3, 4, 5, 6]` at the start.
            PUSH(0),
            LOD,
            PUSH(1),
            LOD,
            PUSH(2),
            LOD,
            PUSH(3),
            LOD,
            PUSH(4),
            LOD,
            PUSH(5),
            LOD,
            // Check that they're equal.
            PUSH(1),
            PUSH(2),
            PUSH(3),
            PUSH(4),
            PUSH(5),
            PUSH(6),
            // a `len` for `EqRange`.
            PUSH(6), // EqRange len
            EQRA,
            HLT,
        ])
        .collect(),
    );

    let a_ca = content_addr(&a);
    let b_ca = content_addr(&b);
    let c_ca = content_addr(&c);

    let node = |program_address, edge_start| Node {
        program_address,
        edge_start,
        reads: Reads::Pre, // unused for this test.
    };
    let nodes = vec![
        node(a_ca.clone(), 0),
        node(b_ca.clone(), 1),
        node(c_ca.clone(), Edge::MAX),
    ];
    let edges = vec![2, 2];
    let predicate = Predicate { nodes, edges };
    let contract = Contract::without_salt(vec![predicate]);
    let pred_addr = PredicateAddress {
        contract: content_addr(&contract),
        predicate: content_addr(&contract.predicates[0]),
    };

    // Create a solution that "solves" our predicate.
    let solution = Solution {
        data: vec![SolutionData {
            predicate_to_solve: pred_addr.clone(),
            decision_variables: Default::default(),
            state_mutations: vec![],
        }],
    };

    // First, validate both predicates and solution.
    essential_check::predicate::check(&contract.predicates[0]).unwrap();
    essential_check::solution::check(&solution).unwrap();

    // There's only one predicate to solve.
    let predicate = Arc::new(contract.predicates[0].clone());
    let get_predicate = |addr: &PredicateAddress| {
        assert_eq!(&pred_addr, addr);
        predicate.clone()
    };
    let programs: HashMap<ContentAddress, Arc<Program>> = vec![
        (a_ca, Arc::new(a)),
        (b_ca, Arc::new(b)),
        (c_ca, Arc::new(c)),
    ]
    .into_iter()
    .collect();
    let get_program: Arc<HashMap<_, _>> = Arc::new(programs);

    // Run the check, and ensure ok and gas aren't 0.
    let gas = solution::check_predicates(
        &State::EMPTY,
        &State::EMPTY,
        Arc::new(solution),
        get_predicate,
        get_program,
        Arc::new(solution::CheckPredicateConfig::default()),
    )
    .await
    .unwrap();

    assert!(gas > 0);
}

// A simple test to check that transient nodes can read state and provide the results to its
// children.
//
// In this program:
//
// 1. *a* pushes a key to the stack.
// 2. *b* uses the key to read from pre *and* post-state (under different nodes).
// 3. *c* multiples the values together and checks they equal 42.
//
//
// ```ignore
//         a
//       /   \
//      /     \
//     /       \
//    /         \
//   v           v
// b (pre)     b (post)
//    \         /
//     \       /
//      \     /
//       \   /
//        \ /
//         v
//         c
// ```
#[tokio::test]
async fn predicate_graph_state_read() {
    use essential_vm::asm::short::*;
    let _ = tracing_subscriber::fmt::try_init();

    let key = vec![9, 9, 9, 9];

    // Push the key and prepare the stack for the key read.
    let a = Program(
        asm::to_bytes(key.iter().map(|&w| PUSH(w)).chain([
            // Push the length and num keys to read for the `KeyRange` op.
            PUSH(4),
            PUSH(1),
            HLT,
        ]))
        .collect(),
    );
    // Perform the read op to read the value from state onto the stack.
    // FIXME: This will change with state slot removal.
    let b = Program(
        asm::to_bytes([
            // Allocate space for reading in [index, len, value].
            // ALOC returns `0` on the stack, i.e. the `mem_addr` to read into.
            PUSH(3),
            ALOC,
            // Read the key range into memory.
            KRNG,
            // Read the value from memory (i.e from `[index, len, value]`) onto the stack.
            PUSH(2),
            LOD,
            // Clear our memory - future programs don't need it.
            PUSH(0),
            FREE,
            // Remove the index, we're only reading one key.
            // POP,
            HLT,
        ])
        .collect(),
    );
    // Stack should now have `[6, 7]` at the start.
    // The `6` from pre-state, the `7` from post-state.
    let c = Program(asm::to_bytes([MUL, PUSH(42), EQ]).collect());

    let a_ca = content_addr(&a);
    let b_ca = content_addr(&b);
    let c_ca = content_addr(&c);

    let node = |program_address, edge_start, reads| Node {
        program_address,
        edge_start,
        reads,
    };
    let nodes = vec![
        node(a_ca.clone(), 0, Reads::Pre),
        node(b_ca.clone(), 2, Reads::Pre),
        node(b_ca.clone(), 3, Reads::Post),
        node(c_ca.clone(), Edge::MAX, Reads::Pre),
    ];
    let edges = vec![1, 2, 3, 3];
    let predicate = Predicate { nodes, edges };
    let contract = Contract::without_salt(vec![predicate]);
    let pred_addr = PredicateAddress {
        contract: content_addr(&contract),
        predicate: content_addr(&contract.predicates[0]),
    };

    // Create the state. The initial state should be 6.
    let mut pre_state = State::EMPTY;
    pre_state.deploy_namespace(pred_addr.contract.clone());
    pre_state.set(pred_addr.contract.clone(), &key, vec![6]);

    // Create a solution that "solves" our predicate.
    let solution = Solution {
        data: vec![SolutionData {
            predicate_to_solve: pred_addr.clone(),
            decision_variables: Default::default(),
            state_mutations: vec![
                // Set the post state to 7.
                Mutation {
                    key,
                    value: vec![7],
                },
            ],
        }],
    };

    // Apply the solution's mutations for the post state.
    let mut post_state = pre_state.clone();
    post_state.apply_mutations(&solution);

    // First, validate both predicates and solution.
    essential_check::predicate::check(&contract.predicates[0]).unwrap();
    essential_check::solution::check(&solution).unwrap();

    // There's only one predicate to solve.
    let predicate = Arc::new(contract.predicates[0].clone());
    let get_predicate = |addr: &PredicateAddress| {
        assert_eq!(&pred_addr, addr);
        predicate.clone()
    };
    let programs: HashMap<ContentAddress, Arc<Program>> = vec![
        (a_ca, Arc::new(a)),
        (b_ca, Arc::new(b)),
        (c_ca, Arc::new(c)),
    ]
    .into_iter()
    .collect();
    let get_program: Arc<HashMap<_, _>> = Arc::new(programs);

    // Run the check, and ensure ok and gas aren't 0.
    let gas = solution::check_predicates(
        &pre_state,
        &post_state,
        Arc::new(solution),
        get_predicate,
        get_program,
        Arc::new(solution::CheckPredicateConfig::default()),
    )
    .await
    .unwrap();

    assert!(gas > 0);
}
