use std::collections::HashSet;

use asm::Op;
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
        asm::Stack::Push(1).into(),        // 1
        asm::Stack::Push(0).into(),        // 1, 0
        asm::Access::RepeatDecVar.into(),  // 1
        asm::Access::RepeatCounter.into(), // 1, counter
        asm::Stack::Push(0).into(),        // 1, counter, 0
        asm::Access::DecisionVar.into(),   // 1, counter, 2
        asm::Stack::Swap.into(),           // 1, 2, counter
        asm::Alu::Sub.into(),              // 1, 2 - counter
        asm::Stack::Push(1).into(),        // 1, 2 - counter, 1
        asm::Alu::Add.into(),              // 1, (2 - counter) + 1
        asm::Stack::Dup.into(),            // 1, (2 - counter) + 1, (2 - counter) + 1
        asm::Access::DecisionVar.into(),   // 1, (2 - counter) + 1, elem
        asm::Stack::Push(2).into(),        // 1, (2 - counter) + 1, elem, 2
        asm::Alu::Mul.into(),              // 1, (2 - counter) + 1, elem * 2
        asm::Stack::Swap.into(),           // 1, elem * 2, (2 - counter) + 1
        asm::Stack::Push(0).into(),        // 1, elem * 2, (2 - counter) + 1, 0
        asm::Access::DecisionVar.into(),   // 1, elem * 2, (2 - counter) + 1, 2
        asm::Alu::Add.into(),              // 1, elem * 2, (2 - counter) + 1 + 2
        asm::Access::DecisionVar.into(),   // 1, elem * 2, out[(2 - counter) + 1 + 2]
        asm::Pred::Eq.into(),
        asm::Pred::And.into(),
        asm::Stack::RepeatEnd.into(), // elem_0, elem_1
    ];
    let res = eval_ops(ops, access).unwrap();
    assert!(res)
}

fn for_in_dec_var() -> Vec<Op> {
    vec![
        asm::Access::DecisionVar.into(),   // len
        asm::Access::RepeatCounter.into(), // len, counter
        asm::Alu::Sub.into(),              // len - counter
        asm::Stack::Push(1).into(),        // len - counter, 1
        asm::Alu::Add.into(),              // (len - counter) + 1
        asm::Access::DecisionVar.into(),   // elem[(len - counter) + 1]
    ]
}

#[test]
fn test_filter_in_asm() {
    let mutable_keys = HashSet::with_capacity(0);
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                intent_to_solve: IntentAddress {
                    set: ContentAddress([0; 32]),
                    intent: ContentAddress([0; 32]),
                },
                decision_variables: vec![
                    DecisionVariable::Inline(3),
                    DecisionVariable::Inline(4),
                    DecisionVariable::Inline(5),
                    DecisionVariable::Inline(6),
                    DecisionVariable::Inline(2),
                    DecisionVariable::Inline(4),
                    DecisionVariable::Inline(6),
                ],
            }],
            index: 0,
            mutable_keys: &mutable_keys,
        },
        state_slots: StateSlots::EMPTY,
    };
    // let len: int;
    // let list: int[len];
    // let even_len: int;
    // let even: int[even_len];
    //
    // constraint {
    //     tmp index: int = 0;
    //     tmp reduce: bool = true;
    //     for i in 0..len where list[i] % 2 == 0 {
    //         acc.reduce &= even[acc.index] == list[i];
    //         acc.index += 1;
    //     }
    //     reduce
    // }
    let mut ops = vec![
        asm::Stack::Push(0).into(),  // 0
        asm::Temporary::Push.into(), //
        asm::Stack::Push(1).into(),  // 1
        asm::Temporary::Push.into(), //
        asm::Stack::Push(0).into(),
        asm::Access::RepeatDecVar.into(),
    ];
    ops.push(asm::Stack::Push(30).into()); // jump to
    ops.push(asm::Stack::Push(0).into());
    ops.extend(for_in_dec_var()); // jump_to, elem[(len - counter) + 1]
    ops.extend(&[
        asm::Stack::Push(2).into(),
        asm::Alu::Mod.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
        asm::TotalControlFlow::JumpForwardIf.into(),
        asm::Stack::Push(1).into(),
        // even[acc.index]
        asm::Stack::Push(0).into(),
        asm::Access::DecisionVar.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Access::DecisionVar.into(), // even[acc.index]
    ]);
    // list[i]
    ops.push(asm::Stack::Push(0).into());
    ops.extend(for_in_dec_var());
    ops.extend(&[
        asm::Pred::Eq.into(),
        asm::Stack::Push(1).into(),
        asm::Temporary::Load.into(),
        asm::Pred::And.into(),
        asm::Temporary::Store.into(),
        // acc.index += 1
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Temporary::Store.into(),
        asm::Stack::RepeatEnd.into(),
        asm::Stack::Push(1).into(),
        asm::Temporary::Load.into(),
    ]);

    let res = eval_ops(&ops, access).unwrap();
    assert!(res)
}

#[test]
fn test_sum_in_asm() {
    let mutable_keys = HashSet::with_capacity(0);
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                intent_to_solve: IntentAddress {
                    set: ContentAddress([0; 32]),
                    intent: ContentAddress([0; 32]),
                },
                decision_variables: vec![
                    DecisionVariable::Inline(3),
                    DecisionVariable::Inline(4),
                    DecisionVariable::Inline(5),
                    DecisionVariable::Inline(6),
                    DecisionVariable::Inline(10),
                ],
            }],
            index: 0,
            mutable_keys: &mutable_keys,
        },
        state_slots: StateSlots::EMPTY,
    };
    // let len: int;
    // let list: int[len];
    // let sum: int;
    //
    // constraint {
    //     tmp reduce: int = 0;
    //     for i in 0..len where list[i] % 2 == 0 {
    //          reduce += list[i];
    //     }
    //     sum == reduce
    // }
    let mut ops = vec![
        asm::Stack::Push(0).into(),
        asm::Temporary::Push.into(),
        asm::Stack::Push(0).into(),
        asm::Access::RepeatDecVar.into(),
    ];
    ops.push(asm::Stack::Push(13).into()); // jump to
    ops.push(asm::Stack::Push(0).into());
    ops.extend(for_in_dec_var());
    ops.extend(&[
        asm::Stack::Push(2).into(),
        asm::Alu::Mod.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
        asm::TotalControlFlow::JumpForwardIf.into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
    ]);
    ops.extend(for_in_dec_var());
    ops.extend(&[
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Alu::Add.into(),
        asm::Temporary::Store.into(),
        asm::Stack::RepeatEnd.into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Stack::Push(4).into(),
        asm::Access::DecisionVar.into(),
        asm::Pred::Eq.into(),
    ]);
    let res = eval_ops(&ops, access).unwrap();
    assert!(res)
}
