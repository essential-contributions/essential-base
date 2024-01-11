use intent_server::check::Directive;
use intent_server::check::Solution;
use intent_server::check::SolvedIntent;
use intent_server::op::Access;
use intent_server::op::Op;
use intent_server::op::Pred;
use intent_server::state_read::Slot;
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
    ];
    let constraints = serde_json::to_vec(&constraints).unwrap();
    let constraints = vec![constraints];

    let intent = Intent {
        state_read,
        state_slots: vec![
            Slot {
                index: 0,
                amount: 4,
                fn_name: "foo".to_string(),
                params: (),
            },
            Slot {
                index: 4,
                amount: 5,
                fn_name: "bar".to_string(),
                params: (),
            },
        ],
        constraints,
        directive: Directive::Satisfy,
    };

    let solved_intent = SolvedIntent {
        intent,
        solution: Solution {
            state_mutations: vec![(6, Some(2))],
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
