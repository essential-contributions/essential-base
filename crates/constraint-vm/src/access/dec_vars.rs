use super::test_utils::ops;
use super::*;
use crate::exec_ops;
use crate::test_util::test_empty_keys;
use crate::test_util::test_transient_data;
use asm::Op;
use essential_constraint_asm as asm;
use essential_types::ContentAddress;
use essential_types::PredicateAddress;
use test_case::test_case;
use test_utils::assert_err;
use test_utils::assert_stack_ok;

#[test_case(
    &[0, 0, 2], &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[3, 99])
    ; "Sanity"
)]
#[test_case(
    &[1, 0, 2], &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[4, 61])
    ; "slot_ix 1"
)]
#[test_case(
    &[1, 1, 1], &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[61])
    ; "slot_ix 1 index 1 len 1"
)]
#[test_case(
    &[1, 1, 0], &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[])
    ; "empty"
)]
#[test_case(
    &[], &[&[3, 99], &[4, 61, 100]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::DecVarLen)))
    ; "missing len"
)]
#[test_case(
    &[1], &[&[3, 99], &[4, 61, 100]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::DecVarValueIx)))
    ; "missing value_ix"
)]
#[test_case(
    &[0, 1], &[&[3, 99], &[4, 61, 100]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::DecVarSlotIx)))
    ; "missing slot_ix"
)]
#[test_case(
    &{
        let mut v = vec![1; Stack::SIZE_LIMIT];
        *v.get_mut(Stack::SIZE_LIMIT - 1).unwrap() = 5;
        v
    },  &[&[], &[3; 6]] =>
    using assert_err!(OpError::Stack(StackError::Overflow))
    ; "values over flow the stack"
)]
#[test_case(
    &[-1, 0, 1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::DecisionSlotIxOutOfBounds(-1)))
    ; "negative slot_ix"
)]
#[test_case(
    &[1, -1, 1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::InvalidAccessRange))
    ; "negative value_ix"
)]
#[test_case(
    &[0, 0, -1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::InvalidAccessRange))
    ; "negative len"
)]
#[test_case(
    &[1, 0, 1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::DecisionSlotIxOutOfBounds(1)))
    ; "slot_ix out of bounds"
)]
#[test_case(
    &[0, 1, 1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::DecisionValueRangeOutOfBounds(1, 2)))
    ; "value ix out of bounds"
)]
#[test_case(
    &[0, 0, 2], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::DecisionValueRangeOutOfBounds(0, 2)))
    ; "len out of bounds"
)]
fn test_dec_var(stack: &[Word], dec_vars: &[&[Word]]) -> OpResult<Vec<Word>> {
    let mut s = Stack::default();
    s.extend(stack.to_vec()).unwrap();

    let dec_vars = dec_vars.iter().map(|v| v.to_vec()).collect::<Vec<_>>();
    decision_var(&dec_vars, &mut s).map(|_| s.into())
}

#[test_case(
    &[0], &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[2])
    ; "Sanity"
)]
#[test_case(
    &[1], &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[3])
    ; "slot_ix 1"
)]
#[test_case(
    &[1], &[&[3, 99], &[]] => using assert_stack_ok(&[0])
    ; "empty"
)]
#[test_case(
    &[], &[&[3, 99], &[4, 61, 100]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::DecVarSlotIx)))
    ; "missing slot_ix"
)]
#[test_case(
    &[-1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::DecisionSlotIxOutOfBounds(-1)))
    ; "negative slot_ix"
)]
#[test_case(
    &[1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::DecisionSlotIxOutOfBounds(1)))
    ; "slot_ix out of bounds"
)]
fn test_dec_var_len(stack: &[Word], dec_vars: &[&[Word]]) -> OpResult<Vec<Word>> {
    let mut s = Stack::default();
    s.extend(stack.to_vec()).unwrap();

    let dec_vars = dec_vars.iter().map(|v| v.to_vec()).collect::<Vec<_>>();
    decision_var_len(&dec_vars, &mut s).map(|_| s.into())
}

#[test_case(
    ops![
        asm::Stack::Push(0),
        asm::Stack::Push(0),
        asm::Stack::Push(2),
        asm::Access::DecisionVar,
    ],
    &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[3, 99])
    ; "sanity dec var"
)]
#[test_case(
    ops![
        asm::Stack::Push(1),
        asm::Stack::Push(1),
        asm::Stack::Push(1),
        asm::Access::DecisionVar,
    ],
    &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[61])
    ; "slot_ix 1, value_ix 1, len 1"
)]
#[test_case(
    ops![
        asm::Stack::Push(1),
        asm::Access::DecisionVarLen,
    ],
    &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[3])
    ; "sanity dec var len"
)]
fn test_dec_var_ops(ops: Vec<Op>, dec_vars: &[&[Word]]) -> OpResult<Vec<Word>> {
    let dec_vars = dec_vars.iter().map(|v| v.to_vec()).collect::<Vec<_>>();
    let data = [SolutionData {
        predicate_to_solve: PredicateAddress {
            contract: ContentAddress([0; 32]),
            predicate: ContentAddress([0; 32]),
        },
        decision_variables: dec_vars,
        state_mutations: vec![],
        transient_data: vec![],
    }];
    let access = Access {
        solution: SolutionAccess {
            data: &data,
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: test_transient_data(),
        },
        state_slots: StateSlots::EMPTY,
    };
    exec_ops(&ops, access)
        .map_err(|e| match e {
            crate::error::ConstraintError::InvalidEvaluation(_) => unreachable!(),
            crate::error::ConstraintError::Op(_, e) => e,
        })
        .map(|stack| stack.into())
}
