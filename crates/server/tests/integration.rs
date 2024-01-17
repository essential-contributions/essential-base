use std::hash::Hash;
use std::hash::Hasher;

use intent_server::check::Directive;
use intent_server::check::SolvedIntent;
use intent_server::check::Transition;
use intent_server::data::InputMessage;
use intent_server::data::Slots;
use intent_server::intent::Intent;
use intent_server::state_read::StateSlot;
use intent_server::state_read::StateSlots;
use intent_server::state_read::VmCall;
use intent_server::Server;
use state_asm::constraint_asm::*;
use state_asm::*;

#[test]
fn sanity() {
    // let path = concat!(
    //     env!("CARGO_MANIFEST_DIR"),
    //     "/../../target/wasm32-unknown-unknown/release/test_state_read.wasm"
    // );

    // let state_read = StateRead::Wasm(std::fs::read(path).unwrap());
    let foo = vec![
        StateReadOp::Constraint(Op::Push(20)),
        StateReadOp::Memory(Memory::Alloc),
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Push(10)),
        StateReadOp::State(State::StateReadWordRange),
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let state_read = vec![foo];
    let state_read = serde_json::to_vec(&state_read).unwrap();

    let constraints = vec![
        Op::Push(3),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(6),
        Op::Pred(Pred::Eq),
        Op::Push(3),
        Op::Push(1),
        Op::Access(Access::State),
        Op::Push(4),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
        Op::Push(4),
        Op::Push(0),
        Op::Access(Access::StateIsSome),
        Op::Pred(Pred::Not),
        Op::Pred(Pred::And),
        Op::Push(5),
        Op::Push(0),
        Op::Access(Access::StateIsSome),
        Op::Pred(Pred::And),
        Op::Push(500),
        Op::Push(0),
        Op::Access(Access::StateIsSome),
        Op::Pred(Pred::Not),
        Op::Pred(Pred::And),
        Op::Push(5),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(20),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
    ];
    let constraints = serde_json::to_vec(&constraints).unwrap();
    let constraints = vec![constraints];

    let intent = Intent {
        slots: Slots {
            decision_variables: 1,
            state: StateSlots::new(vec![
                StateSlot {
                    index: 0,
                    amount: 4,
                    call: VmCall { index: 0 },
                },
                StateSlot {
                    index: 4,
                    amount: 5,
                    call: VmCall { index: 1 },
                },
            ]),
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let solved_intent = SolvedIntent {
        intent,
        solution: Transition {
            state_mutations: vec![([6, 0, 0, 0], Some(2))],
            decision_variables: vec![2],
            ..Default::default()
        },
    };

    let mut server = Server::new();
    for i in 0..15 {
        server.db().stage([0, 0, 0, 0], [i, 0, 0, 0], i.into());
    }
    server.db().stage([0, 0, 0, 0], [14, 0, 0, 0], None);
    server.db().commit();

    let solution = server.check_individual(solved_intent, 1).unwrap();
    assert!(solution);
}

#[test]
fn constrain_dec_vars() {
    let constraints = vec![
        Op::Push(0),
        Op::Access(Access::DecisionVar),
        Op::Push(1),
        Op::Access(Access::DecisionVar),
        Op::Push(2),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
    ];
    let constraints = serde_json::to_vec(&constraints).unwrap();
    let constraints = vec![constraints];
    let intent = Intent {
        slots: Slots {
            decision_variables: 2,
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
            decision_variables: vec![1, 2],
            ..Default::default()
        },
    };

    let solution = server.check_individual(solved_intent, 1).unwrap();
    assert!(solution);
}

// // Erc 20 transfer
// // state sender_bal = state.get(msg.sender)
// // state receiver_bal = state.get(msg.receiver)
// //
// // constraint sender_bal >= msg.amount
// // constraint sender_bal - sender_bal' == msg.amount
// // constraint receiver_bal' - receiver_bal == msg.amount
// #[test]
// fn erc20_transfer() {
//     // let path = concat!(
//     //     env!("CARGO_MANIFEST_DIR"),
//     //     "/../../target/wasm32-unknown-unknown/release/test_erc20.wasm"
//     // );

//     // let state_read = StateRead::Wasm(std::fs::read(path).unwrap());
//     let get_42 = vec![StateReadOp::Constraint(Op::Push(4))];
//     let state_read = vec![get_42];
//     let state_read = serde_json::to_vec(&state_read).unwrap();
//     let mut constraints = vec![];
//     // constraint sender_bal >= msg.amount
//     let constraint = vec![
//         Op::Push(0),
//         Op::Push(0),
//         Op::Access(Access::State),
//         Op::Push(1),
//         Op::Push(0),
//         Op::Access(Access::InputMsgArgWord),
//         Op::Pred(Pred::Gte),
//     ];
//     let constraint = serde_json::to_vec(&constraint).unwrap();
//     constraints.push(constraint);

//     // constraint sender_bal - sender_bal' == msg.amount
//     let constraint = vec![
//         Op::Push(0),
//         Op::Push(0),
//         Op::Access(Access::State),
//         Op::Push(0),
//         Op::Push(1),
//         Op::Access(Access::State),
//         Op::Alu(Alu::Sub),
//         Op::Push(1),
//         Op::Push(0),
//         Op::Access(Access::InputMsgArgWord),
//         Op::Pred(Pred::Eq),
//     ];
//     let constraint = serde_json::to_vec(&constraint).unwrap();
//     constraints.push(constraint);

//     // constraint receiver_bal' - receiver_bal == msg.amount
//     let constraint = vec![
//         Op::Push(1),
//         Op::Push(1),
//         Op::Access(Access::State),
//         Op::Push(1),
//         Op::Push(0),
//         Op::Access(Access::State),
//         Op::Alu(Alu::Sub),
//         Op::Push(1),
//         Op::Push(0),
//         Op::Access(Access::InputMsgArgWord),
//         Op::Pred(Pred::Eq),
//     ];
//     let constraint = serde_json::to_vec(&constraint).unwrap();
//     constraints.push(constraint);

//     let intent = Intent {
//         slots: Slots {
//             state: StateSlots::new(vec![
//                 StateSlot {
//                     index: 0,
//                     amount: 1,
//                     call: VmCall { index: 0 },
//                 },
//                 StateSlot {
//                     index: 1,
//                     amount: 1,
//                     call: VmCall { index: 0 },
//                 },
//             ]),
//             input_message_args: Some(vec![8, 1]),
//             ..Default::default()
//         },
//         state_read,
//         constraints,
//         directive: Directive::Satisfy,
//     };

//     let mut hasher = std::collections::hash_map::DefaultHasher::new();
//     vec![2u64; 8].hash(&mut hasher);
//     let key1 = hasher.finish();
//     let key1 = [key1, key1, key1, key1];

//     let mut hasher = std::collections::hash_map::DefaultHasher::new();
//     vec![1u64; 8].hash(&mut hasher);
//     let key2 = hasher.finish();
//     let key2 = [key2, key2, key2, key2];

//     let solved_intent = SolvedIntent {
//         intent,
//         solution: Transition {
//             input_message: Some(InputMessage {
//                 sender: [2; 4],
//                 args: vec![vec![1; 8], vec![500]],
//             }),
//             state_mutations: vec![(key1, Some(500)), (key2, Some(500))],
//             ..Default::default()
//         },
//     };

//     let mut server = Server::new();
//     let address = [0, 0, 0, 0];
//     server.db().stage(address, key1, Some(1000));
//     server.db().stage(address, key2, Some(0));
//     server.db().commit();
//     let solution = server.check_individual(solved_intent, 1).unwrap();
//     assert!(solution);
// }
