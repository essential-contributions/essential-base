use asm::short::*;
use essential_types::{ContentAddress, PredicateAddress, Solution};
use essential_vm::sync::step_op;
use essential_vm::{asm, Access, GasLimit, Op, ProgramControlFlow, Vm};
use std::sync::Arc;

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
fn test_throughput() {
    let short_n = std::env::var("NUM_SHORT_PROGRAMS")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap();
    let long_n = std::env::var("NUM_LONG_PROGRAMS")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap();
    let access = Access {
        solutions: Arc::new(vec![Solution {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress([0; 32]),
                predicate: ContentAddress([0; 32]),
            },
            predicate_data: vec![vec![2]],
            state_mutations: vec![],
        }]),
        index: 0,
    };
    let mut vm = Vm::default();

    let ops = short_program();

    let mut out = true;
    let s = std::time::Instant::now();
    let op_gas_cost = &|_: &Op| 1;
    let gas_limit = GasLimit::UNLIMITED;
    for _ in 0..short_n {
        for op in &ops {
            step_op(
                access.clone(),
                *op,
                &mut vm,
                &State::EMPTY,
                ops.as_slice(),
                op_gas_cost,
                gas_limit,
            )
            .unwrap();
        }
        out &= vm.stack[0] == 1;
        vm.stack.pop().unwrap();
    }
    let elapsed = s.elapsed();
    assert!(out);
    println!("Short: {:?}, {:?} per run", elapsed, elapsed / short_n);

    let ops = long_program();

    let mut vm = Vm::default();
    let mut out = true;
    let s = std::time::Instant::now();
    let op_gas_cost = &|_: &Op| 1;
    let gas_limit = GasLimit::UNLIMITED;
    for _ in 0..long_n {
        while vm.pc < ops.len() {
            let op = &ops[vm.pc];
            let r = step_op(
                access.clone(),
                *op,
                &mut vm,
                &State::EMPTY,
                ops.as_slice(),
                op_gas_cost,
                gas_limit,
            )
            .unwrap();
            match r {
                Some(ProgramControlFlow::Pc(p)) => vm.pc = p,
                Some(ProgramControlFlow::Halt) => break,
                Some(ProgramControlFlow::ComputeEnd) => {
                    vm.pc += 1;
                    break;
                }
                Some(ProgramControlFlow::ComputeResult(_)) => break,
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
