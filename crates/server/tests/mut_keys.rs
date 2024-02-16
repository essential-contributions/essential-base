use essential_types::{
    intent::{Directive, Intent},
    slots::Slots,
    solution::{KeyMutation, Mutation, Sender, Solution, SolutionData, StateMutation},
    SourceAddress,
};
use intent_server::{intent::ToIntentAddress, Server};
use state_asm::{
    constraint_asm::{Access, Pred},
    Op,
};

#[test]
fn test_mut_keys() {
    let mut constraints = vec![];
    let constraint = vec![
        // let keys: int[2] = context::mut_keys();
        Op::Push(2),
        Op::Access(Access::MutKeysLen),
        Op::Pred(Pred::Eq),
        // constraint keys[0] == [0, 0, 0, 1];
        Op::Push(0),
        Op::Access(Access::MutKeys), // [1, 0, 0, 0, 1]
        Op::Push(1),                // [1, 0, 0, 0, 1, 1]
        Op::Pred(Pred::Eq),         // [1, 0, 0, 0, 1]
        Op::Swap,                   // [1, 0, 0, 1, 0]
        Op::Push(0),                // [1, 0, 0, 1, 0, 0]
        Op::Pred(Pred::Eq),         // [1, 0, 0, 1, 1]
        Op::Pred(Pred::And),        // [1, 0, 0, 1]
        Op::Swap,
        Op::Push(0),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
        Op::Swap,
        Op::Push(0),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
        Op::Pred(Pred::And),
        // constraint keys[1] == [0, 0, 0, 2];
        Op::Push(1),
        Op::Access(Access::MutKeys),
        Op::Push(2),
        Op::Pred(Pred::Eq),
        Op::Swap,
        Op::Push(0),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
        Op::Swap,
        Op::Push(0),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
        Op::Swap,
        Op::Push(0),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
        Op::Pred(Pred::And),
    ];
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    let persistent = Intent {
        slots: Default::default(),
        state_read: Default::default(),
        constraints,
        directive: Directive::Satisfy,
    };

    let transient = Intent {
        slots: Slots {
            permits: 1,
            ..Default::default()
        },
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    };

    let mut server = Server::new();

    let deployed_address = server.deploy_intent_set(vec![persistent.clone()]).unwrap();
    server.submit_intent(transient.clone()).unwrap();

    let good_solution = Solution {
        data: vec![
            SolutionData {
                intent_to_solve: SourceAddress::transient(transient.intent_address()),
                decision_variables: Default::default(),
                sender: Sender::eao([0; 4]),
            },
            SolutionData {
                intent_to_solve: SourceAddress::persistent(
                    deployed_address.into(),
                    persistent.intent_address(),
                ),
                decision_variables: Default::default(),
                sender: Sender::transient([0; 4], transient.intent_address()),
            },
        ],
        state_mutations: vec![StateMutation {
            address: deployed_address.into(),
            mutations: vec![
                Mutation::Key(KeyMutation {
                    key: [0, 0, 0, 1],
                    value: Some(42),
                }),
                Mutation::Key(KeyMutation {
                    key: [0, 0, 0, 2],
                    value: Some(43),
                }),
            ],
        }],
    };

    let utility = server.submit_solution(good_solution.clone()).unwrap();
    assert_eq!(utility, 2);

    let mut bad_solution = good_solution.clone();
    bad_solution.state_mutations[0]
        .mutations
        .push(Mutation::Key(KeyMutation {
            key: [0, 0, 0, 3],
            value: Some(44),
        }));
    server.submit_solution(bad_solution).unwrap_err();

    let mut bad_solution = good_solution.clone();
    match &mut bad_solution.state_mutations[0].mutations[0] {
        Mutation::Key(ref mut key_mutation) => key_mutation.key = [0, 0, 0, 3],
        _ => unreachable!(),
    }
    server.submit_solution(bad_solution).unwrap_err();
}
