use essential_asm as asm;
use essential_types::{solution::Solution, ContentAddress, PredicateAddress};
use essential_vm::{sync::eval_ops, Access};
use std::sync::Arc;

mod util;
use util::*;

#[test]
fn test_forall_in_asm() {
    #[cfg(feature = "tracing")]
    let _ = tracing_subscriber::fmt::try_init();
    let access = Access {
        solutions: Arc::new(vec![Solution {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress([0; 32]),
                predicate: ContentAddress([0; 32]),
            },
            predicate_data: vec![vec![2], vec![4, 6], vec![8, 12]],
            state_mutations: vec![],
        }]),
        index: 0,
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
        asm::Access::PredicateData.into(),
        // count up true
        asm::Stack::Push(1).into(),
        // repeat len times
        asm::Stack::Repeat.into(),
        // (1, counter..(counter + 1)) = list[i]
        asm::Stack::Push(1).into(),
        asm::Access::RepeatCounter.into(),
        asm::Stack::Push(1).into(),
        asm::Access::PredicateData.into(),
        // list[i] * 2
        asm::Stack::Push(2).into(),
        asm::Alu::Mul.into(),
        // (2, counter..(counter + 1)) = out[i]
        asm::Stack::Push(2).into(),
        asm::Access::RepeatCounter.into(),
        asm::Stack::Push(1).into(),
        asm::Access::PredicateData.into(),
        // out[i] == list[i] * 2
        asm::Pred::Eq.into(),
        // true AND out[0] == list[0] * 2 ... AND out[len - 1] == list[len - 1] * 2
        asm::Pred::And.into(),
        asm::Stack::RepeatEnd.into(),
    ];
    let res = eval_ops(ops, access, &State::EMPTY).unwrap();
    assert!(res)
}

#[test]
fn test_fold_filter_in_asm() {
    let access = Access {
        solutions: Arc::new(vec![Solution {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress([0; 32]),
                predicate: ContentAddress([0; 32]),
            },
            predicate_data: vec![],
            state_mutations: vec![],
        }]),
        index: 0,
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
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(),
        // for i in 0..3 unrolled
        // acc += list[0];
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
        // acc += list[1];
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
        // acc += list[2];
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(3).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
        // sum == acc;
        asm::Stack::Push(6).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Pred::Eq.into(),
    ];
    let res = eval_ops(ops, access.clone(), &State::EMPTY).unwrap();
    assert!(res);

    let ops = &[
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(),
        asm::Stack::Push(11).into(), // Num to jump
        asm::Stack::Push(1).into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Mod.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
        asm::TotalControlFlow::JumpIf.into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
        asm::Stack::Push(11).into(), // Num to jump
        asm::Stack::Push(2).into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Mod.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
        asm::TotalControlFlow::JumpIf.into(),
        asm::Stack::Push(2).into(),
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
        asm::Stack::Push(11).into(), // Num to jump
        asm::Stack::Push(3).into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Mod.into(),
        asm::Stack::Push(0).into(),
        asm::Pred::Eq.into(),
        asm::Pred::Not.into(),
        asm::TotalControlFlow::JumpIf.into(),
        asm::Stack::Push(3).into(),
        asm::Stack::Push(1).into(),
        asm::Memory::Alloc.into(),
        asm::Memory::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(1).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
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
    let res = eval_ops(ops, access, &State::EMPTY).unwrap();
    assert!(res)
}
