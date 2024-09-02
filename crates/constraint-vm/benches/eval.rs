use std::collections::{HashMap, HashSet};

use asm::Op;
use criterion::{criterion_group, criterion_main, Criterion};
use essential_constraint_asm as asm;
use essential_constraint_vm::{eval_bytecode_iter, Access, Gas, SolutionAccess, StateSlots};
use essential_types::{solution::SolutionData, ContentAddress, PredicateAddress};

pub fn bench(c: &mut Criterion) {
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
    let bytes = [asm::Stack::Push(1).into(), asm::Stack::Pop.into()];
    for i in [100, 1000, 10_000, 100_000] {
        let mut bytes: Vec<Op> = bytes.iter().cycle().take(i).copied().collect();
        bytes.push(asm::Stack::Push(1).into());
        let bytes: Vec<_> = asm::to_bytes(bytes).collect();
        c.bench_function(&format!("push_pop_{}", i), |b| {
            b.iter(|| eval_bytecode_iter(bytes.iter().copied(), access, &|_: &Op| 1, Gas::MAX))
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
