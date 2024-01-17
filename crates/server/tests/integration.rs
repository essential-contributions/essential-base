use intent_server::check::Directive;
use intent_server::data::Slots;
use intent_server::intent::Intent;
use intent_server::solution::Solution;
use intent_server::state_read::StateSlot;
use intent_server::state_read::StateSlots;
use intent_server::state_read::VmCall;
use intent_server::Server;

#[test]
fn sanity() {
    let mut server = Server::new();

    // let state_read: Vec<StateReadOp> = vec![
    // ];

    let state_read = serde_json::to_vec(&vec![state_read]).unwrap();
    let intent = Intent {
        slots: Slots {
            decision_variables: 1,
            state: StateSlots::new(vec![StateSlot {
                index: 0,
                amount: 1,
                call: VmCall { index: 0 },
            }]),
            ..Default::default()
        },
        state_read,
        constraints: todo!(),
        directive: Directive::Satisfy,
    };
    let solution = Solution {
        transitions: todo!(),
        state_mutations: todo!(),
    };
    let intent_address = server.submit_intent(intent).unwrap();
    let utility = server.submit_solution(solution).unwrap();
    assert_eq!(utility, 1);
}
