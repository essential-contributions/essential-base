use std::collections::{HashMap, HashSet};

use criterion::{criterion_group, criterion_main, Criterion};
use essential_constraint_asm as asm;
use essential_constraint_vm::{eval_bytecode_iter, Access, SolutionAccess, StateSlots};
use essential_types::{solution::SolutionData, ContentAddress, IntentAddress};

pub fn bench(c: &mut Criterion) {
    let mutable_keys = HashSet::with_capacity(0);
    let transient_data = HashMap::with_capacity(0);
    let access = Access {
        solution: SolutionAccess {
            data: &[SolutionData {
                intent_to_solve: IntentAddress {
                    set: ContentAddress([0; 32]),
                    intent: ContentAddress([0; 32]),
                },
                decision_variables: vec![],
            }],
            index: 0,
            mutable_keys: &mutable_keys,
            transient_data: &transient_data,
        },
        state_slots: StateSlots::EMPTY,
    };
    let bytes: Vec<_> = asm::to_bytes([
        asm::Stack::Push(1).into(),
        asm::Stack::Pop.into(),
        asm::Stack::Push(1).into(),
    ])
    .collect();
    let mut iter = bytes.into_iter().cycle();
    c.bench_function("push_pop", |b| {
        b.iter(|| eval_bytecode_iter(iter.by_ref().take(100), access))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
