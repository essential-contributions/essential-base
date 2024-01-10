use intent_server::Server;
use intent_server::check::Directive;
use intent_server::check::Solution;
use intent_server::check::SolvedIntent;
use intent_server::state_read::Slot;
use intent_server::Intent;

#[test]
fn sanity() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/wasm32-unknown-unknown/release/test_state_read.wasm"
    );

    let state_read = std::fs::read(path).unwrap();

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
        constraints: vec![],
        directive: Directive::Satisfy,
    };

    let solved_intent = SolvedIntent {
        intent,
        solution: Solution {
            state_mutations: vec![(0, Some(1))], 
            ..Default::default()
        },
    };

    let mut server = Server::new();
    for i in 0..15 {
        server.db().stage(i, i.into());
    }
    server.db().commit();
    
    let solution = server.check(solved_intent, 1).unwrap();
    assert!(solution);
}
