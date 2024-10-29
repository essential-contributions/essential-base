use essential_check::{predicate, solution};
use essential_hash::content_addr;
use essential_state_read_vm as state_read_vm;
use essential_types::{
    contract::Contract,
    predicate::{Edge, Node, Predicate, Program, Reads},
    solution::{Mutation, Solution, SolutionData},
    ContentAddress, PredicateAddress, Word,
};
use std::{collections::HashMap, sync::Arc};
use util::{empty_solution, predicate_addr, State};

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

// Tests a predicate for contractting slot 0 to 42 against its associated solution.
#[tokio::test]
async fn check_predicate_42_with_solution() {
    let (programs, predicates, solution) = util::test_predicate_42_solution_pair(1, [0; 32]);

    // First, validate both predicates and solution.
    predicate::check_signed_contract(&predicates).unwrap();
    solution::check(&solution).unwrap();

    // Construct the pre state, then apply mutations to acquire post state.
    let mut pre_state = State::EMPTY;
    pre_state.deploy_namespace(essential_hash::content_addr(&predicates.contract));
    let mut post_state = pre_state.clone();
    post_state.apply_mutations(&solution);

    // There's only one predicate to solve.
    let predicate_addr = predicate_addr(&predicates, 0);
    let predicate = Arc::new(predicates.contract[0].clone());
    let get_predicate = |addr: &PredicateAddress| {
        assert_eq!(&predicate_addr, addr);
        predicate.clone()
    };
    let programs: HashMap<_, _> = programs
        .into_iter()
        .map(|p| (content_addr(&p), Arc::new(p)))
        .collect();
    let get_program = Arc::new(programs);

    // Run the check, and ensure ok and gas isn't 0.
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

// A simple test to check that resulting stacks are passed from parents to children.
//
// ```ignore
// a    b
//  \  /
//   \/
//    c
// ```
#[tokio::test]
async fn predicate_with_two_parents_one_leaf() {
    tracing_subscriber::fmt::init();
    let a = Program(
        state_read_vm::asm::to_bytes([
            state_read_vm::asm::Stack::Push(1).into(),
            state_read_vm::asm::Stack::Push(2).into(),
            state_read_vm::asm::Stack::Push(3).into(),
            state_read_vm::asm::TotalControlFlow::Halt.into(),
        ])
        .collect(),
    );
    let b = Program(
        state_read_vm::asm::to_bytes([
            state_read_vm::asm::Stack::Push(4).into(),
            state_read_vm::asm::Stack::Push(5).into(),
            state_read_vm::asm::Stack::Push(6).into(),
            state_read_vm::asm::TotalControlFlow::Halt.into(),
        ])
        .collect(),
    );
    let c = Program(
        state_read_vm::asm::to_bytes([
            // Stack should already have `[1, 2, 3, 4, 5, 6]`.
            state_read_vm::asm::Stack::Push(1).into(),
            state_read_vm::asm::Stack::Push(2).into(),
            state_read_vm::asm::Stack::Push(3).into(),
            state_read_vm::asm::Stack::Push(4).into(),
            state_read_vm::asm::Stack::Push(5).into(),
            state_read_vm::asm::Stack::Push(6).into(),
            // a `len` for `EqRange`.
            state_read_vm::asm::Stack::Push(6).into(), // EqRange len
            state_read_vm::asm::Pred::EqRange.into(),
            state_read_vm::asm::TotalControlFlow::Halt.into(),
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
