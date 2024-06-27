use std::collections::{HashMap, HashSet};

use essential_constraint_asm as asm;
use essential_constraint_vm::{eval_ops, Access, SolutionAccess, StateSlots};
use essential_types::{solution::SolutionData, ContentAddress, PredicateAddress};

#[test]
fn test_forall_in_asm() {
    let mutable_keys = HashSet::with_capacity(0);
    let transient_data = HashMap::with_capacity(0);
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                predicate_to_solve: PredicateAddress {
                    contract: ContentAddress([0; 32]),
                    predicate: ContentAddress([0; 32]),
                },
                decision_variables: vec![vec![2], vec![4], vec![6], vec![8], vec![12]],
                state_mutations: vec![],
                transient_data: vec![],
            }],
            index: 0,
            mutable_keys: &mutable_keys,
            transient_data: &transient_data,
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

#[test]
fn test_fold_filter_in_asm() {
    let mutable_keys = HashSet::with_capacity(0);
    let transient_data = HashMap::with_capacity(0);
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                predicate_to_solve: PredicateAddress {
                    contract: ContentAddress([0; 32]),
                    predicate: ContentAddress([0; 32]),
                },
                decision_variables: vec![],
                state_mutations: vec![],
                transient_data: vec![],
            }],
            index: 0,
            mutable_keys: &mutable_keys,
            transient_data: &transient_data,
        },
        state_slots: StateSlots::EMPTY,
    };

    // let list: int[3] = [1, 2, 3];
    // let even: int[1] = [2];
    // let sum: int = 6;
    //
    // constraint {
    //   tmp acc: int = 0;
    //   for i in 0..3 {
    //     acc += list[i];
    //   }
    //   sum == acc;
    // };
    let ops = &[
        // tmp acc: int = 0;
        asm::Stack::Push(1).into(),
        asm::Temporary::Alloc.into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Store.into(),
        // for i in 0..3 unrolled
        // acc += list[0];
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Temporary::Store.into(),
        // acc += list[1];
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Add.into(),
        asm::Temporary::Store.into(),
        // acc += list[2];
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Stack::Push(3).into(),
        asm::Alu::Add.into(),
        asm::Temporary::Store.into(),
        // sum == acc;
        asm::Stack::Push(6).into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Pred::Eq.into(),
    ];
    let res = eval_ops(ops, access).unwrap();
    assert!(res);

    // constraint {
    //   tmp acc: int[] = [];
    //   tmp count: int = 0;
    //   for i in 0..3 {
    //     if list[i] % 2 == 0 {
    //       acc.push(list[i]);
    //       count += 1;
    //    }
    //    count == 1 && even == acc;
    // }
    let ops = &[
        // tmp count: int = 0;
        asm::Stack::Push(1).into(),
        asm::Temporary::Alloc.into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Store.into(),
        // for i in 0..3 unrolled
        // if list[0] % 2 == 0
        asm::Stack::Push(11).into(), // Num to jump
        asm::Stack::Push(1).into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Mod.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
        asm::TotalControlFlow::JumpForwardIf.into(),
        // acc.push(list[0]);
        asm::Stack::Push(1).into(),
        asm::Temporary::Alloc.into(),
        asm::Stack::Push(1).into(),
        asm::Temporary::Store.into(),
        // count += 1;
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Temporary::Store.into(),
        // if list[1] % 2 == 0
        asm::Stack::Push(11).into(), // Num to jump
        asm::Stack::Push(2).into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Mod.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
        asm::TotalControlFlow::JumpForwardIf.into(),
        // acc.push(list[1]);
        asm::Stack::Push(1).into(),
        asm::Temporary::Alloc.into(),
        asm::Stack::Push(2).into(),
        asm::Temporary::Store.into(),
        // count += 1;
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Temporary::Store.into(),
        // if list[2] % 2 == 0
        asm::Stack::Push(11).into(), // Num to jump
        asm::Stack::Push(3).into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Mod.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
        asm::TotalControlFlow::JumpForwardIf.into(),
        // acc.push(list[2]);
        asm::Stack::Push(1).into(),
        asm::Temporary::Alloc.into(),
        asm::Stack::Push(3).into(),
        asm::Temporary::Store.into(),
        // count += 1;
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Temporary::Store.into(),
        // count == 1 && even == acc;
        asm::Stack::Push(0).into(),
        asm::Temporary::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Pred::Eq.into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(1).into(),
        asm::Temporary::Load.into(),
        asm::Pred::Eq.into(),
        asm::Pred::And.into(),
    ];
    let res = eval_ops(ops, access).unwrap();
    assert!(res)
}
