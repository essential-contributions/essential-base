use std::hash::Hash;
use std::hash::Hasher;

use intent_server::check::Directive;
use intent_server::check::Solution;
use intent_server::check::SolvedIntent;
use intent_server::data::InputMessage;
use intent_server::data::Slots;
use intent_server::op::Access;
use intent_server::op::Alu;
use intent_server::op::Op;
use intent_server::op::Pred;
use intent_server::state_read::StateSlot;
use intent_server::Intent;
use intent_server::Server;

#[test]
fn sanity() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/wasm32-unknown-unknown/release/test_state_read.wasm"
    );

    let state_read = std::fs::read(path).unwrap();

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

    let get_param_one = vec![Op::Push(0), Op::Access(Access::DecisionVar)];
    let get_param_one = serde_json::to_vec(&get_param_one).unwrap();

    let intent = Intent {
        slots: Slots {
            decision_variables: 1,
            state: vec![
                StateSlot {
                    index: 0,
                    amount: 4,
                    fn_name: "foo".to_string(),
                    params: vec![],
                },
                StateSlot {
                    index: 4,
                    amount: 5,
                    fn_name: "bar".to_string(),
                    params: vec![get_param_one],
                },
            ],
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let solved_intent = SolvedIntent {
        intent,
        solution: Solution {
            state_mutations: vec![(6, Some(2))],
            decision_variables: vec![2],
            ..Default::default()
        },
    };

    let mut server = Server::new();
    for i in 0..15 {
        server.db().stage(i, i.into());
    }
    server.db().stage(14, None);
    server.db().commit();

    let solution = server.check(solved_intent, 1).unwrap();
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
        state_read: vec![],
        constraints,
        directive: Directive::Satisfy,
    };

    let mut server = Server::new();

    let solved_intent = SolvedIntent {
        intent,
        solution: Solution {
            decision_variables: vec![1, 2],
            ..Default::default()
        },
    };

    let solution = server.check(solved_intent, 1).unwrap();
    assert!(solution);
}

// Erc 20 transfer
// state sender_bal = state.get(msg.sender)
// state receiver_bal = state.get(msg.receiver)
//
// constraint sender_bal >= msg.amount
// constraint sender_bal - sender_bal' == msg.amount
// constraint receiver_bal' - receiver_bal == msg.amount
#[test]
fn erc20_transfer() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/wasm32-unknown-unknown/release/test_erc20.wasm"
    );

    let state_read = std::fs::read(path).unwrap();
    let mut constraints = vec![];
    // constraint sender_bal >= msg.amount
    let constraint = vec![
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(1),
        Op::Push(0),
        Op::Access(Access::InputMsgArgWord),
        Op::Pred(Pred::Gte),
    ];
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    // constraint sender_bal - sender_bal' == msg.amount
    let constraint = vec![
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(0),
        Op::Push(1),
        Op::Access(Access::State),
        Op::Alu(Alu::Sub),
        Op::Push(1),
        Op::Push(0),
        Op::Access(Access::InputMsgArgWord),
        Op::Pred(Pred::Eq),
    ];
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    // constraint receiver_bal' - receiver_bal == msg.amount
    let constraint = vec![
        Op::Push(1),
        Op::Push(1),
        Op::Access(Access::State),
        Op::Push(1),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Alu(Alu::Sub),
        Op::Push(1),
        Op::Push(0),
        Op::Access(Access::InputMsgArgWord),
        Op::Pred(Pred::Eq),
    ];
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    let get_msg_sender = vec![Op::Access(Access::InputMsgSender)];
    let get_msg_receiver = vec![Op::Push(0), Op::Access(Access::InputMsgArg)];
    let get_msg_sender = serde_json::to_vec(&get_msg_sender).unwrap();
    let get_msg_receiver = serde_json::to_vec(&get_msg_receiver).unwrap();

    let intent = Intent {
        slots: Slots {
            state: vec![
                StateSlot {
                    index: 0,
                    amount: 1,
                    fn_name: "get_sender_bal".to_string(),
                    params: vec![get_msg_sender],
                },
                StateSlot {
                    index: 1,
                    amount: 1,
                    fn_name: "get_receiver_bal".to_string(),
                    params: vec![get_msg_receiver],
                },
            ],
            input_message_args: vec![8, 1],
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    vec![2u64; 8].hash(&mut hasher);
    let key1 = hasher.finish();

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    vec![1u64; 8].hash(&mut hasher);
    let key2 = hasher.finish();

    let solved_intent = SolvedIntent {
        intent,
        solution: Solution {
            input_message: InputMessage {
                sender: [2; 8],
                args: vec![vec![1; 8], vec![500]],
            },
            state_mutations: vec![(key1, Some(500)), (key2, Some(500))],
            ..Default::default()
        },
    };

    let mut server = Server::new();
    server.db().stage(key1, Some(1000));
    server.db().stage(key2, Some(0));
    server.db().commit();
    let solution = server.check(solved_intent, 1).unwrap();
    assert!(solution);
}
