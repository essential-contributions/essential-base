use std::collections::HashSet;

use asm::short::*;
use essential_types::{ContentAddress, PredicateAddress, Solution};
use essential_vm::sync::step_op_sync;
use essential_vm::{asm, Access, GasLimit, Op, OpKind, Vm};

mod util;

use util::*;

fn short_program() -> Vec<Op> {
    vec![PUSH(0), PUSH(0), PUSH(1), DATA, PUSH(1), GTE]
}

fn long_program() -> Vec<Op> {
    vec![
        PUSH(1),
        PUSH(5000),
        PUSH(1),
        REP,
        PUSH(0),
        PUSH(0),
        PUSH(1),
        DATA,
        PUSH(1),
        GTE,
        AND,
        REPE,
    ]
}

#[test]
fn test_sync_throughput() {
    let short_n = std::env::var("NUM_SHORT_PROGRAMS")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap();
    let long_n = std::env::var("NUM_LONG_PROGRAMS")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap();
    let mutable_keys = HashSet::with_capacity(0);
    let access = Access {
        solutions: &[Solution {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress([0; 32]),
                predicate: ContentAddress([0; 32]),
            },
            predicate_data: vec![vec![2]],
            state_mutations: vec![],
        }],
        index: 0,
        mutable_keys: &mutable_keys,
    };
    let mut vm = Vm::default();

    let ops = short_program()
        .into_iter()
        .map(OpKind::from)
        .map(|op| match op {
            OpKind::Sync(s) => s,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();

    let mut out = true;
    let s = std::time::Instant::now();
    for _ in 0..short_n {
        for op in &ops {
            step_op_sync(*op, access, &mut vm).unwrap();
        }
        out &= vm.stack[0] == 1;
        vm.stack.pop().unwrap();
    }
    let elapsed = s.elapsed();
    assert!(out);
    println!("Short: {:?}, {:?} per run", elapsed, elapsed / short_n);

    let ops = long_program()
        .into_iter()
        .map(OpKind::from)
        .map(|op| match op {
            OpKind::Sync(s) => s,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();

    let mut vm = Vm::default();
    let mut out = true;
    let s = std::time::Instant::now();
    for _ in 0..long_n {
        while vm.pc < ops.len() {
            let op = &ops[vm.pc];
            let r = step_op_sync(*op, access, &mut vm).unwrap();
            match r {
                Some(p) => vm.pc = p,
                None => break,
            }
        }
        out &= vm.stack[0] == 1;
        vm.stack.pop().unwrap();
        vm.pc = 0;
    }
    let elapsed = s.elapsed();
    assert!(out);
    println!("Long: {:?}, {:?} per run", elapsed, elapsed / long_n);
}

#[tokio::test]
async fn test_async_throughput() {
    let short_n = std::env::var("NUM_SHORT_PROGRAMS")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap();
    let long_n = std::env::var("NUM_LONG_PROGRAMS")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap();
    let mutable_keys = HashSet::with_capacity(0);
    let access = Access {
        solutions: &[Solution {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress([0; 32]),
                predicate: ContentAddress([0; 32]),
            },
            predicate_data: vec![vec![2]],
            state_mutations: vec![],
        }],
        index: 0,
        mutable_keys: &mutable_keys,
    };
    let state = State::EMPTY;
    let mut vm = Vm::default();

    let ops = short_program();

    let mut out = true;
    let s = std::time::Instant::now();

    for _ in 0..short_n {
        vm.exec_ops(&ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
            .await
            .unwrap();
        out &= vm.stack[0] == 1;
    }

    let elapsed = s.elapsed();
    assert!(out);
    println!(
        "Short async: {:?}, {:?} per run",
        elapsed,
        elapsed / short_n
    );

    let mut vm = Vm::default();

    let ops = long_program();

    let mut out = true;
    let s = std::time::Instant::now();
    for _ in 0..long_n {
        vm.exec_ops(&ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
            .await
            .unwrap();
        out &= vm.stack[0] == 1;
        vm.stack.pop().unwrap();
        vm.pc = 0;
    }

    let elapsed = s.elapsed();
    assert!(out);
    println!("Long async: {:?}, {:?} per run", elapsed, elapsed / long_n);
}
