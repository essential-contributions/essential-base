use intent_server::check::Directive;
use intent_server::check::Solution;
use intent_server::check::SolvedIntent;
use intent_server::data::Slots;
use intent_server::op::Access;
use intent_server::op::Op;
use intent_server::op::Pred;
use intent_server::state_read::vm::ControlFlow;
use intent_server::state_read::vm::Memory;
use intent_server::state_read::vm::StateReadOp;
use intent_server::state_read::StateRead;
use intent_server::state_read::StateSlot;
use intent_server::state_read::VmCall;
use intent_server::Intent;
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
        StateReadOp::StateReadWordRange,
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
            state: StateRead::Vm(vec![StateSlot {
                index: 0,
                amount: 1,
                call: VmCall { index: 0 },
            }]),
            ..Default::default()
        },
        state_read: StateRead::Vm(state_read),
        constraints,
        directive: Directive::Satisfy,
    };

    let mut server = Server::new();

    let solved_intent = SolvedIntent {
        intent,
        solution: Solution {
            ..Default::default()
        },
    };

    server.db().stage([14, 14, 14, 14], Some(42));
    server.db().commit();

    let solution = server.check(solved_intent, 1).unwrap();
    assert!(solution);
}

// Extern state reads
#[test]
fn extern_state_reads() {}

// Message outputs

// hash sizes
