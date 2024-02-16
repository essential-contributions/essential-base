#![cfg(feature = "yurt")]

use std::io::Write;

use essential_types::{
    intent::Intent,
    solution::{KeyMutation, Mutation, Sender, Solution, SolutionData, StateMutation},
    IntentAddress, SourceAddress,
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

    let deployed_intent = compile_yurt(code);

    let mut server = Server::new();

    let deployed_address = server
        .deploy_intent_set(vec![deployed_intent.clone()])
        .unwrap();

    let code = r#"
state x: int = storage::get_extern(${deployed_address}, 0x0000000000000000000000000000000000000000000000000000000000000000); // 42
state y: int = storage::get_extern(${deployed_address}, 0x0000000000000000000000000000000000000000000000000000000000000001); // 42
state z: int = storage::get_extern(${deployed_address}, 0x0000000000000000000000000000000000000000000000000000000000000002); // 42

constraint x' - x == 1; // x' == 43
constraint y' == y + 4; // y' == 47
constraint x' + y' > 89 && x' * y' > 1932; 
constraint x < y;
solve satisfy;
"#;
    let code = code.replace(
        "${deployed_address}",
        &format!("0x{}", hex::encode(IntentAddress::from(deployed_address).0)),
    );

    let mut intent = compile_yurt(&code);
    intent.slots.permits = 1;
    let transient_address = intent.intent_address();

    let transitions = [
        SolutionData {
            intent_to_solve: SourceAddress::transient(transient_address.clone()),
            decision_variables: vec![],
            sender: Sender::Eoa([0; 4]),
        },
        SolutionData {
            intent_to_solve: SourceAddress::persistent(
                deployed_address.into(),
                deployed_intent.intent_address(),
            ),
            decision_variables: vec![141, 129, 1936, 0, 1, 4, 9, 16],
            sender: Sender::transient([0; 4], transient_address),
        },
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

#[test]
fn test_erc20() {
    // Transfer
    let code = r#"
state bal: int = storage::get(context::mut_keys(0));
state to_bal: int = storage::get(context::mut_keys(1));

constraint context::mut_keys_len() == 2;
constraint context::mut_keys(0) == context::sender();
constraint to_bal' - to_bal == bal - bal';

solve satisfy;
"#;

    let deployed_intent = compile_yurt(code);

    let mut server = Server::new();

    let deployed_address = server
        .deploy_intent_set(vec![deployed_intent.clone()])
        .unwrap();

    let code = r#"
let to: b256 = 0x0000000000000000000000000000000000000000000000000000000000000001;
let amount: int = 10;
state bal: int = storage::get_extern(${deployed_address}, context::sender());
state to_bal: int = storage::get_extern(${deployed_address}, to);

constraint bal - bal' == amount;
constraint to_bal' - to_bal == amount;

solve satisfy;
"#;
    let code = code.replace(
        "${deployed_address}",
        &format!("0x{}", hex::encode(IntentAddress::from(deployed_address).0)),
    );

    let mut intent = compile_yurt(&code);
    intent.slots.permits = 1;
    let transient_address = intent.intent_address();

    let transitions = [
        SolutionData {
            intent_to_solve: SourceAddress::transient(transient_address.clone()),
            decision_variables: vec![0, 0, 0, 1, 10],
            sender: Sender::Eoa([0; 4]),
        },
        SolutionData {
            intent_to_solve: SourceAddress::persistent(
                deployed_address.into(),
                deployed_intent.intent_address(),
            ),
            decision_variables: vec![],
            sender: Sender::transient([0; 4], transient_address),
        },
    ];

    let solution = Solution {
        data: transitions.into_iter().collect(),
        state_mutations: vec![StateMutation {
            address: deployed_address.into(),
            mutations: vec![
                Mutation::Key(KeyMutation {
                    key: [0, 0, 0, 0],
                    value: Some(90),
                }),
                Mutation::Key(KeyMutation {
                    key: [0, 0, 0, 1],
                    value: Some(10),
                }),
            ],
        }],
    };

    let address: Address = deployed_address;
    server.db().stage(address, [0, 0, 0, 0], Some(100));
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
