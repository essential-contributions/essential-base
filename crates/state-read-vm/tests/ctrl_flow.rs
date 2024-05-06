mod util;

use essential_state_read_vm::{
    asm::{self, Op},
    error::{ControlFlowError, OpSyncError},
    error::{OpError, StateReadError},
    GasLimit, Vm,
};
use util::*;

#[tokio::test]
async fn jump_forward() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(4).into(), // Jump index.
        asm::ControlFlow::Jump.into(),
        asm::ControlFlow::Halt.into(),
        asm::ControlFlow::Halt.into(),
        asm::Stack::Push(6).into(), // Jump destination.
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
        asm::ControlFlow::Halt.into(),
    ];
    let spent = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_op: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
    assert_eq!(spent, 6);
    assert_eq!(&vm.stack[..], &[42]);
}

#[tokio::test]
async fn jump_back() {
    let mut vm = Vm::default();
    // This program continues to loop and multiply by 2 until gas is exhausted.
    let ops = &[
        asm::Stack::Push(1).into(),
        asm::Stack::Push(2).into(), // Jump destination.
        asm::Alu::Mul.into(),
        asm::Stack::Push(1).into(), // Jump index.
        asm::ControlFlow::Jump.into(),
        asm::ControlFlow::Halt.into(), // We'll never reach this.
    ];
    // `Jump` has no condition, so jumping back will cause looping forever.
    // Set a gas limit to ensure that this completes.
    // The total gas cost will be `loop_count * 4`, plus 1 for the first operation.
    // To calculate 2^5, we want to iterate 5 times without the final jump ops:
    // (loop_count * 4) + 1 - final_jump = (5 * 4) + 1 - 2 = 19
    let total = 19;
    let gas_limit = GasLimit {
        per_yield: GasLimit::DEFAULT_PER_YIELD,
        total,
    };
    let res = vm
        .exec_ops(ops, *test_access(), &State::EMPTY, &|_op: &Op| 1, gas_limit)
        .await;
    let err = match res {
        // The failing operations hould be the `Push` that follows the final
        // multiplication, i.e. `3`.
        Err(StateReadError::Op(3, OpError::OutOfGas(err))) => err,
        _ => panic!("expected out of gas error, found {:?}", res),
    };
    assert_eq!(err.spent, total);
    assert_eq!(err.op_gas, 1);
    assert_eq!(err.limit, total);
    // After 5 total iterations, the result should be 2^5.
    assert_eq!(&vm.stack[..], &[32]);
}

#[tokio::test]
async fn jump_if_forward() {
    let mut vm = Vm::default();
    let jump_dest = 10;
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(jump_dest).into(),
        asm::Stack::Push(0).into(),      // false
        asm::ControlFlow::JumpIf.into(), // JumpIf should not jump.
        asm::Stack::Push(7).into(),
        asm::Stack::Push(jump_dest).into(),
        asm::Stack::Push(1).into(),      // true
        asm::ControlFlow::JumpIf.into(), // JumpIf should not jump.
        asm::ControlFlow::Halt.into(),
        asm::ControlFlow::Halt.into(),
        asm::Alu::Mul.into(), // Jump destination.
        asm::ControlFlow::Halt.into(),
    ];
    let spent = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_op: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
    assert_eq!(spent, 10);
    assert_eq!(&vm.stack[..], &[42]);
}

#[tokio::test]
async fn jump_if_back() {
    let mut vm = Vm::default();
    // Find the nth power of 2 by looping using JumpIf.
    let nth_power = 8;
    let jump_dest = 2;
    let ops = &[
        // Setup the loop counter and the value carrying the power of 2.
        asm::Stack::Push(0).into(), // [counter]
        asm::Stack::Push(1).into(), // [counter, value]
        // Multiply value by 2.
        asm::Stack::Push(2).into(), // [counter, value, 2] JUMP DESTINATION
        asm::Alu::Mul.into(),       // [counter, value*2]
        // Increment the counter by 1.
        asm::Stack::Swap.into(),    // [value, counter]
        asm::Stack::Push(1).into(), // [value, counter, 1]
        asm::Alu::Add.into(),       // [value, counter+1]
        // Jump back if the counter is less than `nth_power`.
        asm::Stack::Swap.into(),            // [counter, value]
        asm::Stack::Push(1).into(),         // [counter, value, 1]
        asm::Stack::DupFrom.into(),         // [counter, value, counter]
        asm::Stack::Push(nth_power).into(), // [counter, value, counter, nth_power]
        asm::Pred::Lt.into(),               // [counter, value, cond]
        asm::Stack::Push(jump_dest).into(), // [counter, value, cond, jump_dest]
        asm::Stack::Swap.into(),            // [counter, value, jump_dest, cond]
        asm::ControlFlow::JumpIf.into(),    // [counter, value] JUMP BACK
        // Pop the counter so that we're left with the value.
        asm::Stack::Swap.into(), // [value, counter]
        asm::Stack::Pop.into(),  // [value]
        asm::ControlFlow::Halt.into(),
    ];
    let spent = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_op: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
    // After 5 total iterations, the result should be 2^5.
    assert_eq!(
        spent,
        2 /*setup*/ + 13 * nth_power as u64 /*loop*/ + 3 /*cleanup*/
    );
    assert_eq!(&vm.stack[..], &[2i64.pow(nth_power as u32)]);
}

#[tokio::test]
async fn jump_if_invalid_cond() {
    let mut vm = Vm::default();
    let invalid_cond = 2; // Valid is 0 or 1.
    let ops = &[
        asm::Stack::Push(0).into(), // Destination Index.
        asm::Stack::Push(invalid_cond).into(),
        asm::ControlFlow::JumpIf.into(),
    ];
    let res = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await;
    match res {
        Err(StateReadError::Op(
            _,
            OpError::Sync(OpSyncError::ControlFlow(ControlFlowError::InvalidJumpIfCondition(n))),
        )) if n == invalid_cond => (),
        _ => panic!("expected overflow, found {:?}", res),
    }
}

#[tokio::test]
async fn missing_halt() {
    let mut vm = Vm::default();
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
    ];
    let res = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await;
    match res {
        Err(StateReadError::PcOutOfRange(pc)) if pc == ops.len() => (),
        _ => panic!("expected overflow, found {:?}", res),
    }
}

#[tokio::test]
async fn jump_pc_out_of_range() {
    let mut vm = Vm::default();
    let pc_out_of_range = 3;
    let ops = &[
        asm::Stack::Push(pc_out_of_range).into(),
        asm::ControlFlow::Jump.into(),
        asm::ControlFlow::Halt.into(),
    ];
    let res = vm
        .exec_ops(
            ops,
            *test_access(),
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await;
    match res {
        Err(StateReadError::PcOutOfRange(pc)) if pc == pc_out_of_range as usize => (),
        _ => panic!("expected overflow, found {:?}", res),
    }
}
