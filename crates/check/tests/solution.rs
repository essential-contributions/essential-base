use constraint_vm::asm::Op;
use essential_check::{intent, solution};
use essential_constraint_vm as constraint_vm;
use essential_state_read_vm as state_read_vm;
use essential_types::{
    intent::{Directive, Intent},
    solution::{Mutation, Solution, SolutionData},
    ContentAddress, IntentAddress, Word,
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
        state_mutations: vec![],
        transient_data: vec![],
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
            intent_to_solve: test_intent_addr(),
            decision_variables: vec![vec![0]; (solution::MAX_DECISION_VARIABLES + 1) as usize],
            state_mutations: vec![],
            transient_data: vec![],
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
            intent_to_solve: test_intent_addr(),
            decision_variables: vec![],
            state_mutations: (0..(solution::MAX_STATE_MUTATIONS + 1))
                .map(test_mutation)
                .collect(),
            transient_data: vec![],
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
            intent_to_solve: test_intent_addr(),
            decision_variables: vec![],
            state_mutations: vec![
                Mutation {
                    key: vec![0; 4],
                    value: vec![42],
                };
                2
            ],
            transient_data: vec![],
        }],
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::StateMutations(solution::InvalidStateMutations::MultipleMutationsForSlot(addr, key))
            if addr == test_intent_addr() && key == [0; 4]
    ));
}

#[test]
fn too_many_transient_data() {
    let solution = Solution {
        data: vec![SolutionData {
            intent_to_solve: test_intent_addr(),
            decision_variables: vec![],
            state_mutations: vec![],
            transient_data: (0..(solution::MAX_TRANSIENT_DATA + 1))
                .map(test_mutation)
                .collect(),
        }],
    };
    assert!(matches!(
        solution::check(&solution).unwrap_err(),
        solution::InvalidSolution::TransientData(solution::InvalidTransientData::TooMany(n))
            if n == solution::MAX_TRANSIENT_DATA + 1
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
    let mut pre_state = State::EMPTY;
    pre_state.deploy_namespace(essential_hash::intent_set_addr::from_intents(&intents.set));
    let mut post_state = pre_state.clone();
    post_state.apply_mutations(&solution);

    // There's only one intent to solve.
    let intent_addr = intent_addr(&intents, 0);
    let intent = Arc::new(intents.set[0].clone());
    let get_intent = |addr: &IntentAddress| {
        assert_eq!(&intent_addr, addr);
        intent.clone()
    };

    // Run the check, and ensure util and gas aren't 0.
    let (util, gas) = solution::check_intents(
        &pre_state,
        &post_state,
        Arc::new(solution),
        get_intent,
        Arc::new(solution::CheckIntentConfig::default()),
    )
    .await
    .unwrap();

    // Util should be 1 - only one solved intent.
    assert_eq!(util, 1.0);
    assert!(gas > 0);
}

#[tokio::test]
async fn intent_with_multiple_state_reads_and_slots() {
    let read_three_slots = state_read_vm::asm::to_bytes([
        state_read_vm::asm::Stack::Push(3).into(),
        state_read_vm::asm::StateSlots::AllocSlots.into(),
        state_read_vm::asm::Stack::Push(0).into(), // Key
        state_read_vm::asm::Stack::Push(1).into(), // Key length
        state_read_vm::asm::Stack::Push(1).into(), // Num keys to read
        state_read_vm::asm::Stack::Push(0).into(), // Destination slot
        state_read_vm::asm::StateRead::KeyRange,
        state_read_vm::asm::Stack::Push(1).into(), // Key
        state_read_vm::asm::Stack::Push(1).into(), // Key length
        state_read_vm::asm::Stack::Push(2).into(), // Num keys to read
        state_read_vm::asm::Stack::Push(1).into(), // Destination slot
        state_read_vm::asm::StateRead::KeyRange,
        state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let read_two_slots = state_read_vm::asm::to_bytes([
        state_read_vm::asm::Stack::Push(2).into(),
        state_read_vm::asm::StateSlots::AllocSlots.into(),
        state_read_vm::asm::Stack::Push(3).into(), // Key
        state_read_vm::asm::Stack::Push(1).into(), // Key length
        state_read_vm::asm::Stack::Push(2).into(), // Num keys to read
        state_read_vm::asm::Stack::Push(0).into(), // Destination slot
        state_read_vm::asm::StateRead::KeyRange,
        state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();

    let slot_len = |slot, range, len| -> Vec<Op> {
        vec![
            constraint_vm::asm::Stack::Push(slot).into(),  // slot
            constraint_vm::asm::Stack::Push(range).into(), // range
            constraint_vm::asm::Stack::Push(1).into(),
            constraint_vm::asm::Access::StateLenRange.into(),
            constraint_vm::asm::Stack::Push(len).into(),
            constraint_vm::asm::Pred::Eq.into(),
        ]
    };
    let mut constraints: Vec<Op> = vec![];
    // Slot 0 must have length 5.
    constraints.extend(slot_len(0, 1, 5));

    // Slot 0 must equal 0, 1, 2, 3, 4.
    let c: Vec<Op> = vec![
        constraint_vm::asm::Stack::Push(0).into(), // slot
        constraint_vm::asm::Stack::Push(1).into(),
        constraint_vm::asm::Access::State.into(),
        constraint_vm::asm::Stack::Push(0).into(),
        constraint_vm::asm::Stack::Push(1).into(),
        constraint_vm::asm::Stack::Push(2).into(),
        constraint_vm::asm::Stack::Push(3).into(),
        constraint_vm::asm::Stack::Push(4).into(),
        constraint_vm::asm::Stack::Push(5).into(), // Eq range length
        constraint_vm::asm::Pred::EqRange.into(),
    ];
    constraints.extend(c);
    constraints.push(constraint_vm::asm::Pred::And.into());

    // Slots 1, 2, 3, 4 must have length 1.
    constraints.extend(slot_len(1, 4, 1));
    constraints.push(constraint_vm::asm::Pred::And.into());

    // Slots 1, 2, 3, 4 must be equal to 5, 6, 7, 8.
    let c: Vec<Op> = vec![
        constraint_vm::asm::Stack::Push(1).into(), // slot
        constraint_vm::asm::Stack::Push(4).into(), // range
        constraint_vm::asm::Stack::Push(1).into(),
        constraint_vm::asm::Access::StateRange.into(),
        constraint_vm::asm::Stack::Push(5).into(),
        constraint_vm::asm::Stack::Push(6).into(),
        constraint_vm::asm::Stack::Push(7).into(),
        constraint_vm::asm::Stack::Push(8).into(),
        constraint_vm::asm::Stack::Push(4).into(), // Eq range length
        constraint_vm::asm::Pred::EqRange.into(),
    ];
    constraints.extend(c);
    constraints.push(constraint_vm::asm::Pred::And.into());

    let intent = Intent {
        state_read: vec![read_three_slots, read_two_slots],
        constraints: vec![constraint_vm::asm::to_bytes(constraints).collect()],
        directive: Directive::Satisfy,
    };

    let (sk, _pk) = random_keypair([1; 32]);
    let intents = essential_sign::intent_set::sign(vec![intent], &sk);
    let intent_addr = util::intent_addr(&intents, 0);

    // Create the solution.
    let solution = Solution {
        data: vec![SolutionData {
            intent_to_solve: intent_addr,
            decision_variables: Default::default(),
            state_mutations: vec![
                Mutation {
                    key: vec![0],
                    value: vec![0, 1, 2, 3, 4],
                },
                Mutation {
                    key: vec![1],
                    value: vec![5],
                },
                Mutation {
                    key: vec![2],
                    value: vec![6],
                },
                Mutation {
                    key: vec![3],
                    value: vec![7],
                },
                Mutation {
                    key: vec![4],
                    value: vec![8],
                },
            ],
            transient_data: vec![],
        }],
    };

    // First, validate both intents and solution.
    intent::check_signed_set(&intents).unwrap();
    solution::check(&solution).unwrap();

    // Construct the pre state, then apply mutations to acquire post state.
    let mut pre_state = State::EMPTY;
    pre_state.deploy_namespace(essential_hash::intent_set_addr::from_intents(&intents.set));
    let mut post_state = pre_state.clone();
    post_state.apply_mutations(&solution);

    // There's only one intent to solve.
    let intent_addr = util::intent_addr(&intents, 0);
    let intent = Arc::new(intents.set[0].clone());
    let get_intent = |addr: &IntentAddress| {
        assert_eq!(&intent_addr, addr);
        intent.clone()
    };

    // Run the check, and ensure util and gas aren't 0.
    let (util, gas) = solution::check_intents(
        &pre_state,
        &post_state,
        Arc::new(solution),
        get_intent,
        Arc::new(solution::CheckIntentConfig::default()),
    )
    .await
    .unwrap();

    // Util should be 1 - only one solved intent.
    assert_eq!(util, 1.0);
    assert!(gas > 0);
}
