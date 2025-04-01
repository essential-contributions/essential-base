use essential_asm as asm;
use essential_types::{solution::Solution, ContentAddress, PredicateAddress};
use essential_vm::{sync::eval_ops, Access, GasLimit, Op};
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

    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(0).into(),
        asm::Stack::Push(1).into(),
        asm::Access::PredicateData.into(),
        asm::Stack::Push(1).into(),
        asm::Stack::Repeat.into(),
        asm::Stack::Push(1).into(),
        asm::Access::RepeatCounter.into(),
        asm::Stack::Push(1).into(),
        asm::Access::PredicateData.into(),
        asm::Stack::Push(2).into(),
        asm::Alu::Mul.into(),
        asm::Stack::Push(2).into(),
        asm::Access::RepeatCounter.into(),
        asm::Stack::Push(1).into(),
        asm::Access::PredicateData.into(),
        asm::Pred::Eq.into(),
        asm::Pred::And.into(),
        asm::Stack::RepeatEnd.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let res = eval_ops(ops, access, &State::EMPTY, op_gas_cost, GasLimit::UNLIMITED).unwrap();
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

    let ops = &[
        asm::Stack::Push(0).into(),
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
        asm::Stack::Push(2).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Stack::Push(3).into(),
        asm::Alu::Add.into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Store.into(),
        asm::Stack::Push(6).into(),
        asm::Stack::Push(0).into(),
        asm::Memory::Load.into(),
        asm::Pred::Eq.into(),
    ];
    let op_gas_cost = &|_: &Op| 1;
    let res = eval_ops(
        ops,
        access.clone(),
        &State::EMPTY,
        op_gas_cost,
        GasLimit::UNLIMITED,
    )
    .unwrap();
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
    let op_gas_cost = &|_: &Op| 1;
    let res = eval_ops(ops, access, &State::EMPTY, op_gas_cost, GasLimit::UNLIMITED).unwrap();
    assert!(res)
}
