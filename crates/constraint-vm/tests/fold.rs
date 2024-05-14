use std::collections::HashSet;

use essential_constraint_asm as asm;
use essential_constraint_vm::{eval_ops, Access, SolutionAccess, StateSlots};
use essential_types::{
    solution::{DecisionVariable, SolutionData},
    ContentAddress, IntentAddress,
};

#[test]
fn test_fold_in_asm() {
    let mutable_keys = HashSet::with_capacity(0);
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                intent_to_solve: IntentAddress {
                    set: ContentAddress([0; 32]),
                    intent: ContentAddress([0; 32]),
                },
                decision_variables: vec![
                    DecisionVariable::Inline(2),
                    DecisionVariable::Inline(4),
                    DecisionVariable::Inline(6),
                    DecisionVariable::Inline(8),
                    DecisionVariable::Inline(12),
                ],
            }],
            index: 0,
            mutable_keys: &mutable_keys,
        },
        state_slots: StateSlots::EMPTY,
    };

    // let len: int;
    // let list: int[len];
    // let out: int[len];
    //
    // constraint forall i in 0..len { out[i] == list[i] * 2 };
    let ops = &[
        asm::Stack::Push(1).into(), // 1
        asm::Stack::Push(2).into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Repeat.into(),         // 1
        asm::Access::RepeatCounter.into(), // 1, counter
        asm::Stack::Push(1).into(),        // 1, 1
        asm::Alu::Add.into(),              // 1, counter + 1
        asm::Stack::Dup.into(),            // 1, counter + 1, counter + 1
        asm::Access::DecisionVar.into(),   // 1, counter + 1, elem
        asm::Stack::Push(2).into(),        // 1, counter + 1, elem, 2
        asm::Alu::Mul.into(),              // 1, counter + 1, elem * 2
        asm::Stack::Swap.into(),           // 1, elem * 2, counter + 1
        asm::Stack::Push(0).into(),        // 1, elem * 2, counter + 1, 0
        asm::Access::DecisionVar.into(),   // 1, elem * 2, counter + 1, 2
        asm::Alu::Add.into(),              // 1, elem * 2, counter + 1 + 2
        asm::Access::DecisionVar.into(),   // 1, elem * 2, out[counter + 1 + 2]
        asm::Pred::Eq.into(),
        asm::Pred::And.into(),
        asm::Stack::RepeatEnd.into(), // elem_0, elem_1
    ];
    let res = eval_ops(ops, access).unwrap();
    assert!(res)
}
