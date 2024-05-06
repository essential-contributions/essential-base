use essential_check::{intent, solution};
use essential_sign::sign;
use essential_types::{
    solution::{
        DecisionVariable, DecisionVariableIndex, Mutation, Solution, SolutionData, StateMutation,
    },
    ContentAddress, IntentAddress,
};
use std::sync::Arc;
use util::{empty_solution, intent_addr, random_keypair, State};

pub mod util;

fn test_intent_addr() -> IntentAddress {
    IntentAddress {
        set: ContentAddress([0; 32]),
        intent: ContentAddress([0; 32]),
    }
}

fn test_solution_data() -> SolutionData {
    SolutionData {
        intent_to_solve: test_intent_addr(),
        decision_variables: vec![],
    }
}

fn test_state_mutation() -> StateMutation {
    StateMutation {
        pathway: 0,
        mutations: vec![],
    }
}

#[test]
fn check_signed_solution() {
    let mut solution = empty_solution();
    solution.data = vec![SolutionData {
        intent_to_solve: test_intent_addr(),
        decision_variables: vec![DecisionVariable::Inline(0)],
    }];
    solution.state_mutations = vec![StateMutation {
        pathway: 0,
        mutations: Default::default(),
    }];
    let (sk, _pk) = random_keypair([0xFA; 32]);
    solution.partial_solutions = vec![sign(ContentAddress([0xFF; 32]), sk)];
    let solution = sign(solution, sk);
    solution::check_signed(&solution).unwrap();
}

#[test]
fn invalid_solution_signature() {
    let solution = empty_solution();
    let (sk, _pk) = random_keypair([0; 32]);
    let mut signed = sign(solution, sk);
    signed.signature.0 = [0; 64];
    assert!(matches!(
        solution::check_signed(&signed).unwrap_err(),
        solution::InvalidSignedSolution::Signature(_),
    ));
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
        ..empty_solution()
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
            intent_to_solve: test_intent_addr(),
            decision_variables: vec![
                DecisionVariable::Inline(0);
                (solution::MAX_DECISION_VARIABLES + 1) as usize
            ],
        }],
        ..empty_solution()
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::Data(solution::InvalidSolutionData::TooManyDecisionVariables(0, n))
            if n == solution::MAX_DECISION_VARIABLES as usize + 1
    ));
}

#[test]
fn unresolving_decision_variable() {
    let dec_var_ix = DecisionVariableIndex {
        solution_data_index: 0,
        variable_index: 42,
    };
    let solution = Solution {
        data: vec![SolutionData {
            intent_to_solve: test_intent_addr(),
            decision_variables: vec![DecisionVariable::Transient(dec_var_ix)],
        }],
        ..empty_solution()
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::Data(solution::InvalidSolutionData::UnresolvingDecisionVariable(ix))
            if ix == dec_var_ix
    ));
}

#[test]
fn decision_variables_cycle() {
    let solution = Solution {
        data: vec![SolutionData {
            intent_to_solve: test_intent_addr(),
            decision_variables: vec![
                DecisionVariable::Transient(DecisionVariableIndex {
                    solution_data_index: 0,
                    variable_index: 1,
                }),
                DecisionVariable::Transient(DecisionVariableIndex {
                    solution_data_index: 0,
                    variable_index: 0,
                }),
            ],
        }],
        ..empty_solution()
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::Data(solution::InvalidSolutionData::DecisionVariablesCycle(_))
    ));
}

#[test]
fn decision_variables_cycle_via_data() {
    let solution = Solution {
        data: vec![
            SolutionData {
                intent_to_solve: test_intent_addr(),
                decision_variables: vec![DecisionVariable::Transient(DecisionVariableIndex {
                    solution_data_index: 1,
                    variable_index: 0,
                })],
            },
            SolutionData {
                intent_to_solve: test_intent_addr(),
                decision_variables: vec![DecisionVariable::Transient(DecisionVariableIndex {
                    solution_data_index: 0,
                    variable_index: 0,
                })],
            },
        ],
        ..empty_solution()
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::Data(solution::InvalidSolutionData::DecisionVariablesCycle(_))
    ));
}

#[test]
fn state_mutation_pathways_must_have_associated_solution_data() {
    let solution = Solution {
        state_mutations: vec![StateMutation {
            // Note: pathway out of bounds of solution data to trigger error.
            pathway: 1,
            mutations: Default::default(),
        }],
        data: vec![test_solution_data()],
        ..empty_solution()
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::StateMutations(
            solution::InvalidStateMutations::PathwayOutOfRangeOfSolutionData(1)
        ),
    ));
}

#[test]
fn too_many_state_mutations() {
    let solution = Solution {
        data: vec![test_solution_data()],
        state_mutations: vec![test_state_mutation(); solution::MAX_STATE_MUTATIONS + 1],
        ..empty_solution()
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
        data: vec![test_solution_data()],
        state_mutations: vec![StateMutation {
            pathway: 0,
            mutations: vec![
                Mutation {
                    key: [0; 4],
                    value: Some(42),
                };
                2
            ],
        }],
        ..empty_solution()
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::StateMutations(solution::InvalidStateMutations::MultipleMutationsForSlot(addr, key))
            if addr == test_intent_addr() && key == [0; 4]
    ));
}

#[test]
fn too_many_partial_solutions() {
    let (sk, _pk) = random_keypair([0; 32]);
    let solution = Solution {
        data: vec![test_solution_data()],
        partial_solutions: vec![
            sign(ContentAddress([0; 32]), sk);
            solution::MAX_PARTIAL_SOLUTIONS + 1
        ],
        ..empty_solution()
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::PartialSolutions(solution::InvalidPartialSolutions::TooMany(n))
            if n == solution::MAX_PARTIAL_SOLUTIONS + 1
    ));
}

#[test]
fn invalid_partial_solution_signature() {
    let (sk, _pk) = random_keypair([0; 32]);
    let mut signed = sign(ContentAddress([0; 32]), sk);
    signed.signature.0 = [0; 64];
    let solution = Solution {
        data: vec![test_solution_data()],
        partial_solutions: vec![signed],
        ..empty_solution()
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::PartialSolutions(solution::InvalidPartialSolutions::Signature(
            0,
            _
        ))
    ));
}

// Tests an intent for setting slot 0 to 42 against its associated solution.
#[tokio::test]
async fn check_intent_42_with_solution() {
    let (intents, solution) = util::test_intent_42_solution_pair(1, [0; 32]);

    // First, validate both intents and solution.
    intent::check_signed_set(&intents).unwrap();
    solution::check(&solution).unwrap();

    // Construct the pre state, then apply mutations to acquire post state.
    let pre_state = State::EMPTY;
    let mut post_state = pre_state.clone();
    post_state.apply_mutations(&solution);

    // There's only one intent to solve.
    let intent_addr = intent_addr(&intents, 0);
    let intent = Arc::new(intents.data[0].clone());
    let get_intent = |addr: &IntentAddress| {
        assert_eq!(&intent_addr, addr);
        intent.clone()
    };

    // Run the check, and ensure util and gas aren't 0.
    let (util, gas) =
        solution::check_intents(&pre_state, &post_state, Arc::new(solution), get_intent)
            .await
            .unwrap();

    // Util should be 1 - only one solved intent.
    assert_eq!(util, 1.0);
    assert!(gas > 0);
}

#[test]
fn decision_variables_length_mismatch() {
    let (intents, mut solution) = util::test_intent_42_solution_pair(1, [0; 32]);
    // Push a nonsense decision variable to trigger the error.
    let extra_dec_var = solution.data[0].decision_variables[0].clone();
    solution.data[0].decision_variables.push(extra_dec_var);
    // There's only one intent to solve in this case.
    let intent = Arc::new(intents.data[0].clone());
    solution::check_decision_variable_lengths(&solution, |_| intent.clone()).unwrap_err();
}
