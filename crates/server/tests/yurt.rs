#![cfg(feature = "yurt")]

use std::io::Write;

use essential_types::{
    intent::Intent,
    solution::{InputMessage, KeyMutation, Mutation, Solution, SolutionData, StateMutation},
};
use intent_server::{db::Address, intent::ToIntentAddress, Server};
use tempfile::NamedTempFile;

#[test]
fn test_compile_yurt() {
    let code = r#"
state x: int = storage::get(0x0000000000000000000000000000000000000000000000000000000000000000); // 42
state y: int = storage::get(0x0000000000000000000000000000000000000000000000000000000000000001); // 43
state z: int = storage::get(0x0000000000000000000000000000000000000000000000000000000000000002); // 44

let w: int = x + 99; // 141
let u: int = y * 3;  // 129
let v: int = z * z;  // 1936
let a: int[5];

constraint forall i in 0..4 {
    a[i] == i*i
};
constraint x' - x == 1; // x' == 43
constraint y' == y + 4; // y' == 47
constraint x' + y' > 89 && x' * y' > 1932; 
constraint x < y;
constraint w + u + v == 2206;

solve satisfy;
"#;

    let mut intent = compile_yurt(code);
    let mut server = Server::new();

    intent.slots.input_message_args = Some(vec![]);
    let address = intent.intent_address();
    let deployed_address = server.deploy_intent_set(vec![intent]).unwrap();
    let transitions = [(
        deployed_address.into(),
        SolutionData {
            decision_variables: vec![141, 129, 1936, 0, 1, 4, 9, 16],
            input_message: Some(InputMessage {
                sender: deployed_address.into(),
                recipient: address.clone(),
                args: vec![],
            }),
            output_messages: vec![],
        },
    )];

    let solution = Solution {
        data: transitions.into_iter().collect(),
        state_mutations: vec![StateMutation {
            address: address.clone(),
            mutations: vec![
                Mutation::Key(KeyMutation {
                    key: [0, 0, 0, 0],
                    value: Some(43),
                }),
                Mutation::Key(KeyMutation {
                    key: [0, 0, 0, 1],
                    value: Some(47),
                }),
                Mutation::Key(KeyMutation {
                    key: [0, 0, 0, 2],
                    value: Some(44),
                }),
            ],
        }],
    };

    let address: Address = address.into();
    server.db().stage(address, [0, 0, 0, 0], Some(42));
    server.db().stage(address, [0, 0, 0, 1], Some(43));
    server.db().stage(address, [0, 0, 0, 2], Some(44));
    server.db().commit();

    let utility = server.submit_solution(solution).unwrap();
    assert_eq!(utility, 1);
}

fn compile_yurt(code: &str) -> Intent {
    let mut tmpfile = NamedTempFile::new().unwrap();
    write!(tmpfile.as_file_mut(), "{}", code).unwrap();
    let intent = yurtc::parser::parse_project(tmpfile.path())
        .unwrap()
        .flatten()
        .unwrap()
        .compile()
        .unwrap();
    let intent = yurtc::asm_gen::intent_to_asm(&intent).unwrap();
    let intent = serde_json::to_string(&intent).unwrap();
    serde_json::from_str(&intent).unwrap()
}
