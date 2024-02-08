#![cfg(feature = "yurt")]

use std::io::Write;

use essential_types::{
    intent::Intent,
    solution::{
        InputMessage, KeyMutation, Mutation, OutputMessage, Solution, SolutionData, StateMutation,
    },
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

    let mut deployed_intent = compile_yurt(code);

    let code = r#"
state x: int = storage::get_extern(0xC7F646FFC0E4BB73AD1E919E800486F8725966B4C0E21214A47FAE03FF45797B, 0x0000000000000000000000000000000000000000000000000000000000000000); // 42
state y: int = storage::get_extern(0xC7F646FFC0E4BB73AD1E919E800486F8725966B4C0E21214A47FAE03FF45797B, 0x0000000000000000000000000000000000000000000000000000000000000001); // 42
state z: int = storage::get_extern(0xC7F646FFC0E4BB73AD1E919E800486F8725966B4C0E21214A47FAE03FF45797B, 0x0000000000000000000000000000000000000000000000000000000000000002); // 42

constraint x' - x == 1; // x' == 43
constraint y' == y + 4; // y' == 47
constraint x' + y' > 89 && x' * y' > 1932; 
constraint x < y;
solve satisfy;
"#;

    let mut intent = compile_yurt(code);
    intent.slots.output_messages = 1;
    let transient_address = intent.intent_address();

    let mut server = Server::new();

    deployed_intent.slots.input_message_args = Some(vec![]);
    let deployed_address = server
        .deploy_intent_set(vec![deployed_intent.clone()])
        .unwrap();
    let transitions = [
        (
            transient_address.clone(),
            SolutionData {
                decision_variables: vec![],
                input_message: None,
                output_messages: vec![OutputMessage { args: vec![] }],
            },
        ),
        (
            deployed_address.into(),
            SolutionData {
                decision_variables: vec![141, 129, 1936, 0, 1, 4, 9, 16],
                input_message: Some(InputMessage {
                    sender: transient_address,
                    recipient: deployed_intent.intent_address(),
                    args: vec![],
                }),
                output_messages: vec![],
            },
        ),
    ];

    let solution = Solution {
        data: transitions.into_iter().collect(),
        state_mutations: vec![StateMutation {
            address: deployed_address.into(),
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

    let address: Address = deployed_address;
    server.db().stage(address, [0, 0, 0, 0], Some(42));
    server.db().stage(address, [0, 0, 0, 1], Some(43));
    server.db().stage(address, [0, 0, 0, 2], Some(44));
    server.db().commit();

    server.submit_intent(intent).unwrap();
    let utility = server.submit_solution(solution).unwrap();
    assert_eq!(utility, 2);
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
