use intent_server::check::Directive;
use intent_server::check::SolvedIntent;
use intent_server::check::Transition;
use intent_server::data::OutputMessage;
use intent_server::data::Slots;
use intent_server::intent::Intent;
use intent_server::op::Access;
use intent_server::op::Op;
use intent_server::op::Pred;
use intent_server::state_read::vm::ControlFlow;
use intent_server::state_read::vm::Memory;
use intent_server::state_read::vm::State;
use intent_server::state_read::vm::StateReadOp;
use intent_server::state_read::StateSlot;
use intent_server::state_read::StateSlots;
use intent_server::state_read::VmCall;
use intent_server::Server;

#[test]
fn vm_state_reads() {
    let get_42 = vec![
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Memory(Memory::Alloc),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::State(State::StateReadWordRange),
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let state_read = vec![get_42];
    let state_read = serde_json::to_vec(&state_read).unwrap();

    let constraints = vec![
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(42),
        Op::Pred(Pred::Eq),
    ];
    let constraints = serde_json::to_vec(&constraints).unwrap();
    let constraints = vec![constraints];
    let intent = Intent {
        slots: Slots {
            state: StateSlots::new(vec![StateSlot {
                index: 0,
                amount: 1,
                call: VmCall { index: 0 },
            }]),
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let intent_address = intent.address();

    let mut server = Server::new();

    let solved_intent = SolvedIntent {
        intent,
        solution: Transition {
            ..Default::default()
        },
    };

    server
        .db()
        .stage(intent_address, [14, 14, 14, 14], Some(42));
    server.db().commit();

    let solution = server.check_individual(solved_intent, 1).unwrap();
    assert!(solution);
}

// Extern state reads
#[test]
fn extern_state_reads() {
    let get_42 = vec![
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Memory(Memory::Alloc),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::State(State::StateReadWordRangeExtern),
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let state_read = vec![get_42];
    let state_read = serde_json::to_vec(&state_read).unwrap();

    let constraints = vec![
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(42),
        Op::Pred(Pred::Eq),
    ];
    let constraints = serde_json::to_vec(&constraints).unwrap();
    let constraints = vec![constraints];
    let intent = Intent {
        slots: Slots {
            state: StateSlots::new(vec![StateSlot {
                index: 0,
                amount: 1,
                call: VmCall { index: 0 },
            }]),
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let mut server = Server::new();

    let solved_intent = SolvedIntent {
        intent,
        solution: Transition {
            ..Default::default()
        },
    };

    server.db().stage([1, 1, 1, 1], [14, 14, 14, 14], Some(42));
    server.db().commit();

    let solution = server.check_individual(solved_intent, 1).unwrap();
    assert!(solution);
}

// Message outputs
#[test]
fn message_outputs() {
    let constraints = vec![
        Op::Push(0),
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::OutputMsgArgWord),
        Op::Push(42),
        Op::Pred(Pred::Eq),
    ];
    let constraints = serde_json::to_vec(&constraints).unwrap();
    let constraints = vec![constraints];
    let intent = Intent {
        slots: Slots {
            output_messages_args: vec![vec![1]],
            ..Default::default()
        },
        state_read: Default::default(),
        constraints,
        directive: Directive::Satisfy,
    };

    let mut server = Server::new();

    let solved_intent = SolvedIntent {
        intent,
        solution: Transition {
            output_messages: vec![OutputMessage {
                args: vec![vec![42]],
            }],
            ..Default::default()
        },
    };

    let solution = server.check_individual(solved_intent, 1).unwrap();
    assert!(solution);
}

#[test]
fn naughts_crosses() {
    let mut server = Server::new();

    let get_board = vec![
        StateReadOp::Constraint(Op::Push(9)),
        StateReadOp::Memory(Memory::Alloc),
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Push(9)),
        StateReadOp::State(State::StateReadWordRange),
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let state_read = vec![get_board];
    let state_read = serde_json::to_vec(&state_read).unwrap();

    let mut constraints = vec![];

    let constraint = (0..9)
        .flat_map(|i| {
            [
                // State must be none
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::StateIsSome),
                Op::Pred(Pred::Not),
                // Or 0
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::StateIsSome),
                Op::Push(0),
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::State),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::And),
                Op::Pred(Pred::Or),
                // Or 1
                Op::Push(1),
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::State),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::Or),
            ]
        })
        .collect::<Vec<_>>();
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    let constraint = (0..9)
        .flat_map(|i| {
            [
                // State before must be none
                Op::Push(i),
                Op::Push(0),
                Op::Access(Access::StateIsSome),
                Op::Pred(Pred::Not),
                // or it must have not changed
                Op::Push(i),
                Op::Push(0),
                Op::Access(Access::State),
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::State),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::Or),
            ]
        })
        .collect::<Vec<_>>();
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    let deployed_intent = Intent {
        slots: Slots {
            state: StateSlots::new(vec![StateSlot {
                index: 0,
                amount: 9,
                call: VmCall { index: 0 },
            }]),
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let deployed_address = server.deploy_intent(deployed_intent).unwrap();
}
