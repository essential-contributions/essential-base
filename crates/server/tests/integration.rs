use essential_types::solution::Sender;
use essential_types::solution::SolutionData;
use essential_types::SourceAddress;
use intent_server::check::Directive;
use intent_server::data::Slots;
use intent_server::intent::Intent;
use intent_server::intent::ToIntentAddress;
use intent_server::solution::Solution;
use intent_server::state_read::StateSlot;
use intent_server::Server;
use state_asm::constraint_asm::Access;
use state_asm::constraint_asm::Pred;
use state_asm::ControlFlow;
use state_asm::Memory;
use state_asm::Op;
use state_asm::State;
use state_asm::StateReadOp;

#[test]
fn sanity_happy() {
    let mut server = Server::new();

    let (intent, deployed_address) = sanity_test(&mut server);

    let transitions = [(
        SourceAddress::persistent(deployed_address.into(), intent.intent_address()),
        SolutionData {
            decision_variables: vec![11],
            sender: Sender::Eoa([0; 4]),
        },
    )];

    let solution = Solution {
        data: transitions.into_iter().collect(),
        state_mutations: Default::default(),
    };

    server.db().stage(deployed_address, [1, 1, 1, 1], Some(7));
    server.db().commit();

    server.submit_intent(intent).unwrap();
    let utility = server.submit_solution(solution).unwrap();
    assert_eq!(utility, 1);
}

#[test]
fn sanity_unhappy() {
    let mut server = Server::new();

    let (intent, deployed_address) = sanity_test(&mut server);

    let transitions = [(
        SourceAddress::persistent(deployed_address.into(), intent.intent_address()),
        SolutionData {
            decision_variables: vec![11],
            sender: Sender::Eoa([0; 4]),
        },
    )];

    let solution = Solution {
        data: transitions.into_iter().collect(),
        state_mutations: Default::default(),
    };

    server.db().stage(deployed_address, [1, 1, 1, 1], Some(8)); // not 7
    server.db().commit();

    server.submit_intent(intent).unwrap();
    server
        .submit_solution(solution)
        .expect_err("Constraint failed");
}

fn sanity_test(server: &mut Server) -> (Intent, [u64; 4]) {
    let deployed_intent = Intent {
        slots: Slots {
            ..Default::default()
        },
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    };

    let deployed_address = server.deploy_intent_set(vec![deployed_intent]).unwrap();

    // state foo: int = state.extern.get(extern_address, key, 1);
    let state_read: Vec<StateReadOp> = vec![
        // allocate memory
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Memory(Memory::Alloc),
        // extern_addres
        StateReadOp::Constraint(Op::Push(deployed_address[0])),
        StateReadOp::Constraint(Op::Push(deployed_address[1])),
        StateReadOp::Constraint(Op::Push(deployed_address[2])),
        StateReadOp::Constraint(Op::Push(deployed_address[3])),
        // key
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        // amount
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::State(State::StateReadWordRangeExtern),
        // end of program
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];

    let mut constraints = vec![];
    let constraint: Vec<Op> = vec![
        // constraint foo == 7;
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(7),
        Op::Pred(Pred::Eq),
        // var bar: int = 11;
        // constraint bar == 11;
        Op::Push(0),
        Op::Access(Access::DecisionVar),
        Op::Push(11),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
    ];

    let constraint = serde_json::to_vec(&constraint).unwrap();

    constraints.push(constraint);

    let state_read = serde_json::to_vec(&state_read).unwrap();
    let state_read = vec![state_read];

    let intent = Intent {
        slots: Slots {
            decision_variables: 1,
            state: vec![StateSlot {
                index: 0,
                amount: 1,
                program_index: 0,
            }],
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    (intent, deployed_address)
}
