use std::collections::HashSet;

use criterion::{criterion_group, criterion_main, Criterion};
use essential_asm as asm;
use essential_asm::Op;
use essential_types::{ContentAddress, PredicateAddress, Solution, SolutionSet};
use essential_vm::{Access, GasLimit, Vm};

#[path = "../tests/util.rs"]
mod util;
use util::State;

pub fn bench(c: &mut Criterion) {
    let mutable_keys = HashSet::with_capacity(0);
    let solution_set = SolutionSet {
        solutions: vec![Solution {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress([0; 32]),
                predicate: ContentAddress([0; 32]),
            },
            predicate_data: vec![],
            state_mutations: vec![],
        }],
    };

    let access = Access::new(&solution_set, 0, &mutable_keys);

    let bytes = [asm::Stack::Push(1).into(), asm::Stack::Pop.into()];
    let mut vm = Vm::default();
    for i in [100, 1000, 10_000, 100_000] {
        let mut bytes: Vec<Op> = bytes.iter().cycle().take(i).copied().collect();
        bytes.push(asm::Stack::Push(1).into());
        let bytes: Vec<_> = asm::to_bytes(bytes).collect();
        let op_gas_cost = &|_: &Op| 1;
        c.bench_function(&format!("push_pop_{}", i), |b| {
            b.iter(|| {
                vm.exec_bytecode_iter(
                    bytes.iter().copied(),
                    access,
                    &State::EMPTY,
                    op_gas_cost,
                    GasLimit::UNLIMITED,
                )
            })
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
