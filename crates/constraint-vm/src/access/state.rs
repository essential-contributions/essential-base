use crate::exec_ops;
use crate::test_util::test_empty_keys;
use crate::test_util::test_solution_data_arr;

use super::test_utils::ops;
use crate::error::OpError;
use crate::error::StackError;
use essential_constraint_asm as asm;
use essential_constraint_asm::Op;
use test_case::test_case;
use test_utils::assert_err;
use test_utils::assert_stack_ok;

use super::*;

#[test_case(
    &[0, 0, 2, 0], &[&[3, 99]], &[&[1, 2]] => using assert_stack_ok(&[3, 99])
    ; "Sanity"
)]
#[test_case(
    &[0, 0, 2, 1], &[&[3, 99]], &[&[1, 2]] => using assert_stack_ok(&[1, 2])
    ; "post state"
)]
#[test_case(
    &[1, 0, 1, 0], &[&[3, 99], &[42]], &[&[1, 2], &[67, 92]] => using assert_stack_ok(&[42])
    ; "slot_ix 1 pre state"
)]
#[test_case(
    &[1, 0, 1, 1], &[&[3, 99], &[42]], &[&[1, 2], &[67, 92]] => using assert_stack_ok(&[67])
    ; "slot_ix 1 post state"
)]
#[test_case(
    &[1, 1, 1, 1], &[&[3, 99], &[42]], &[&[1, 2], &[67, 92]] => using assert_stack_ok(&[92])
    ; "value_ix 1"
)]
#[test_case(
    &[1, 1, 0, 1], &[&[3, 99], &[42]], &[&[1, 2], &[67, 92]] => using assert_stack_ok(&[])
    ; "empty"
)]
#[test_case(
    &[], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::StateDelta)))
    ; "missing delta"
)]
#[test_case(
    &[0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::StateLen)))
    ; "missing len"
)]
#[test_case(
    &[1, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::StateValueIx)))
    ; "missing value_ix"
)]
#[test_case(
    &[0, 1, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::StateSlotIx)))
    ; "missing slot_ix"
)]
#[test_case(
    &{
        let mut v = vec![1; Stack::SIZE_LIMIT];
        *v.get_mut(Stack::SIZE_LIMIT - 2).unwrap() = 5;
        v
    }, &[], &[&[], &[3; 6]] =>
    using assert_err!(OpError::Stack(StackError::Overflow))
    ; "values over flow the stack"
)]
#[test_case(
    &[0, 0, 1, 2], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::InvalidStateSlotDelta(2)))
    ; "invalid delta"
)]
#[test_case(
    &[0, 0, 1, -1], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::InvalidStateSlotDelta(-1)))
    ; "negative delta"
)]
#[test_case(
    &[-1, 0, 1, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::StateSlotIxOutOfBounds(-1)))
    ; "negative slot_ix"
)]
#[test_case(
    &[1, -1, 1, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::InvalidAccessRange))
    ; "negative value_ix"
)]
#[test_case(
    &[0, 0, -1, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::InvalidAccessRange))
    ; "negative len"
)]
#[test_case(
    &[1, 0, 1, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::StateSlotIxOutOfBounds(1)))
    ; "slot_ix out of bounds"
)]
#[test_case(
    &[0, 1, 1, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::StateValueRangeOutOfBounds(1, 2)))
    ; "value ix out of bounds"
)]
#[test_case(
    &[0, 0, 2, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::StateValueRangeOutOfBounds(0, 2)))
    ; "len out of bounds"
)]
fn test_state(stack: &[Word], pre: &[&[Word]], post: &[&[Word]]) -> OpResult<Vec<Word>> {
    let mut s = Stack::default();
    s.extend(stack.to_vec()).unwrap();

    let (pre, post) = to_state_slots(pre, post);
    let state_slots = StateSlots {
        pre: &pre,
        post: &post,
    };
    state(state_slots, &mut s).map(|_| s.into())
}

#[test_case(
    &[0, 0], &[&[3, 99]], &[&[1, 2, 4]] => using assert_stack_ok(&[2])
    ; "Sanity"
)]
#[test_case(
    &[0, 1], &[&[3, 99]], &[&[1, 2, 4]] => using assert_stack_ok(&[3])
    ; "post state"
)]
#[test_case(
    &[1, 0], &[&[3, 99], &[42]], &[&[1, 2], &[67, 92]] => using assert_stack_ok(&[1])
    ; "slot_ix 1 pre state"
)]
#[test_case(
    &[1, 1], &[&[3, 99], &[42]], &[&[1, 2], &[67, 92]] => using assert_stack_ok(&[2])
    ; "slot_ix 1 post state"
)]
#[test_case(
    &[], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::StateDelta)))
    ; "missing delta"
)]
#[test_case(
    &[0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::StateSlotIx)))
    ; "missing slot_ix"
)]
#[test_case(
    &[0, 2], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::InvalidStateSlotDelta(2)))
    ; "invalid delta"
)]
#[test_case(
    &[0, -1], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::InvalidStateSlotDelta(-1)))
    ; "negative delta"
)]
#[test_case(
    &[-1, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::StateSlotIxOutOfBounds(-1)))
    ; "negative slot_ix"
)]
#[test_case(
    &[1, 0], &[&[3]], &[&[42]] =>
    using assert_err!(OpError::Access(AccessError::StateSlotIxOutOfBounds(1)))
    ; "slot_ix out of bounds"
)]
fn test_state_len(stack: &[Word], pre: &[&[Word]], post: &[&[Word]]) -> OpResult<Vec<Word>> {
    let mut s = Stack::default();
    s.extend(stack.to_vec()).unwrap();

    let (pre, post) = to_state_slots(pre, post);
    let state_slots = StateSlots {
        pre: &pre,
        post: &post,
    };
    state_len(state_slots, &mut s).map(|_| s.into())
}

#[test_case(
    ops![
        asm::Stack::Push(0),
        asm::Stack::Push(0),
        asm::Stack::Push(2),
        asm::Stack::Push(0),
        asm::Access::State,
    ],
    &[&[3, 99]], &[&[1, 2]] => using assert_stack_ok(&[3, 99])
    ; "sanity state"
)]
#[test_case(
    ops![
        asm::Stack::Push(1),
        asm::Stack::Push(1),
        asm::Stack::Push(2),
        asm::Stack::Push(1),
        asm::Access::State,
    ],
    &[&[3, 99]], &[&[1, 2], &[9, 55, -90]] => using assert_stack_ok(&[55, -90])
    ; "state post slot"
)]
#[test_case(
    ops![
        asm::Stack::Push(0),
        asm::Stack::Push(0),
        asm::Access::StateLen,
    ],
    &[&[3, 99]], &[&[1, 2, 4]] => using assert_stack_ok(&[2])
    ; "sanity state len"
)]
#[test_case(
    ops![
        asm::Stack::Push(1),
        asm::Stack::Push(1),
        asm::Access::StateLen,
    ],
    &[&[3, 99]], &[&[1, 2], &[9, 55, -90]] => using assert_stack_ok(&[3])
    ; "state len post slot"
)]
fn test_state_ops(ops: Vec<Op>, pre: &[&[Word]], post: &[&[Word]]) -> OpResult<Vec<Word>> {
    let (pre, post) = to_state_slots(pre, post);
    let state_slots = StateSlots {
        pre: &pre,
        post: &post,
    };

    let access = Access {
        solution: SolutionAccess {
            data: test_solution_data_arr(),
            index: 0,
            mutable_keys: test_empty_keys(),
        },
        state_slots,
    };
    exec_ops(&ops, access)
        .map_err(|e| match e {
            crate::error::ConstraintError::InvalidEvaluation(_) => unreachable!(),
            crate::error::ConstraintError::Op(_, e) => e,
        })
        .map(|stack| stack.into())
}

fn to_state_slots(pre: &[&[Word]], post: &[&[Word]]) -> (Vec<Vec<Word>>, Vec<Vec<Word>>) {
    let pre = pre.iter().map(|x| x.to_vec()).collect();
    let post = post.iter().map(|x| x.to_vec()).collect();
    (pre, post)
}
