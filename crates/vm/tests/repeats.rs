use std::collections::HashSet;

use essential_asm as asm;
use essential_vm::{constraint::eval_ops, Access};
use essential_types::{solution::SolutionData, ContentAddress, PredicateAddress};

#[test]
fn test_forall_in_asm() {
    #[cfg(feature = "tracing")]
    let _ = tracing_subscriber::fmt::try_init();
    let mutable_keys = HashSet::with_capacity(0);
    let access = Access {
        data: &[SolutionData {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress([0; 32]),
                predicate: ContentAddress([0; 32]),
            },
            decision_variables: vec![vec![2], vec![4, 6], vec![8, 12]],
            state_mutations: vec![],
        }],
        index: 0,
        mutable_keys: &mutable_keys,
    };

    // let len: int;
    // let list: int[len];
    // let out: int[len];
    //
    // constraint forall i in 0..len { out[i] == list[i] * 2 };
    let ops = &[
        // true to AND with
        asm::Stack::Push(1).into(),
        // (0, 0..1) = len
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::Access::DecisionVar.into(),
        // count up true
        asm::Stack::Push(1).into(),
        // repeat len times
        asm::Stack::Repeat.into(),
        // (1, counter..(counter + 1)) = list[i]
        asm::Stack::Push(1).into(),
        asm::Access::RepeatCounter.into(),
        asm::Stack::Push(1).into(),
        asm::Access::DecisionVar.into(),
        // list[i] * 2
        asm::Stack::Push(2).into(),
        asm::Alu::Mul.into(),
        // (2, counter..(counter + 1)) = out[i]
        asm::Stack::Push(2).into(),
        asm::Access::RepeatCounter.into(),
        asm::Stack::Push(1).into(),
        asm::Access::DecisionVar.into(),
        // out[i] == list[i] * 2
        asm::Pred::Eq.into(),
        // true AND out[0] == list[0] * 2 ... AND out[len - 1] == list[len - 1] * 2
        asm::Pred::And.into(),
        asm::Stack::RepeatEnd.into(),
    ];
    let res = eval_ops(ops, access).unwrap();
    assert!(res)
}

#[test]
fn test_fold_filter_in_asm() {
    let mutable_keys = HashSet::with_capacity(0);
    let access = Access {
        data: &[SolutionData {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress([0; 32]),
                predicate: ContentAddress([0; 32]),
            },
            decision_variables: vec![],
            state_mutations: vec![],
        }],
        index: 0,
        mutable_keys: &mutable_keys,
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
        asm::Memory::Alloc.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
        // for i in 0..3 unrolled
        // acc += list[0];
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Memory::Store.into(),
        // acc += list[1];
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Add.into(),
        asm::Memory::Store.into(),
        // acc += list[2];
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(3).into(),
        asm::Alu::Add.into(),
        asm::Memory::Store.into(),
        // sum == acc;
        asm::Stack::Push(6).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
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
        asm::Memory::Alloc.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
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
        asm::Memory::Alloc.into(),
        asm::Stack::Push(1).into(),
        asm::Memory::Store.into(),
        // count += 1;
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Memory::Store.into(),
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
        asm::Memory::Alloc.into(),
        asm::Stack::Push(2).into(),
        asm::Memory::Store.into(),
        // count += 1;
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Memory::Store.into(),
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
        asm::Memory::Alloc.into(),
        asm::Stack::Push(3).into(),
        asm::Memory::Store.into(),
        // count += 1;
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Memory::Store.into(),
        // count == 1 && even == acc;
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Pred::Eq.into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(1).into(),
        asm::Memory::Load.into(),
        asm::Pred::Eq.into(),
        asm::Pred::And.into(),
    ];
    let res = eval_ops(ops, access).unwrap();
    assert!(res)
}
