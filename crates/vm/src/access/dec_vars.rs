use super::*;
use crate::{
    asm,
    error::{ExecError, OpError, OpResult, StackError},
    sync::exec_ops,
    types::{ContentAddress, PredicateAddress},
    utils::EmptyState,
    GasLimit, Op,
};
use test_case::test_case;
use test_utils::{assert_err, assert_stack_ok};

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
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PredDataLen)))
    ; "missing len"
)]
#[test_case(
    &[1], &[&[3, 99], &[4, 61, 100]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PredDataValueIx)))
    ; "missing value_ix"
)]
#[test_case(
    &[0, 1], &[&[3, 99], &[4, 61, 100]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PredDataSlotIx)))
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
    using assert_err!(OpError::Access(AccessError::PredicateDataSlotIxOutOfBounds(-1)))
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
    using assert_err!(OpError::Access(AccessError::PredicateDataSlotIxOutOfBounds(1)))
    ; "slot_ix out of bounds"
)]
#[test_case(
    &[0, 1, 1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::PredicateDataValueRangeOutOfBounds(1, 2)))
    ; "value ix out of bounds"
)]
#[test_case(
    &[0, 0, 2], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::PredicateDataValueRangeOutOfBounds(0, 2)))
    ; "len out of bounds"
)]
fn test_dec_var(stack: &[Word], dec_vars: &[&[Word]]) -> OpResult<Vec<Word>> {
    let mut s = Stack::default();
    s.extend(stack.to_vec()).unwrap();

    let dec_vars = dec_vars.iter().map(|v| v.to_vec()).collect::<Vec<_>>();
    predicate_data(&dec_vars, &mut s).map(|_| s.into())
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
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PredDataSlotIx)))
    ; "missing slot_ix"
)]
#[test_case(
    &[-1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::PredicateDataSlotIxOutOfBounds(-1)))
    ; "negative slot_ix"
)]
#[test_case(
    &[1], &[&[3]] =>
    using assert_err!(OpError::Access(AccessError::PredicateDataSlotIxOutOfBounds(1)))
    ; "slot_ix out of bounds"
)]
fn test_dec_var_len(stack: &[Word], dec_vars: &[&[Word]]) -> OpResult<Vec<Word>> {
    let mut s = Stack::default();
    s.extend(stack.to_vec()).unwrap();

    let dec_vars = dec_vars.iter().map(|v| v.to_vec()).collect::<Vec<_>>();
    predicate_data_len(&dec_vars, &mut s)
        .map(|_| s.into())
        .map_err(From::from)
}

#[test_case(
    vec![
        Op::from(asm::Stack::Push(0)),
        Op::from(asm::Stack::Push(0)),
        Op::from(asm::Stack::Push(2)),
        Op::from(asm::Access::PredicateData),
    ],
    &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[3, 99])
    ; "sanity dec var"
)]
#[test_case(
    vec![
        Op::from(asm::Stack::Push(1)),
        Op::from(asm::Stack::Push(1)),
        Op::from(asm::Stack::Push(1)),
        Op::from(asm::Access::PredicateData),
    ],
    &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[61])
    ; "slot_ix 1, value_ix 1, len 1"
)]
#[test_case(
    vec![
        Op::from(asm::Stack::Push(1)),
        Op::from(asm::Access::PredicateDataLen),
    ],
    &[&[3, 99], &[4, 61, 100]] => using assert_stack_ok(&[3])
    ; "sanity dec var len"
)]
fn test_dec_var_ops(ops: Vec<Op>, dec_vars: &[&[Word]]) -> OpResult<Vec<Word>, String> {
    let dec_vars = dec_vars.iter().map(|v| v.to_vec()).collect::<Vec<_>>();
    let solutions = vec![Solution {
        predicate_to_solve: PredicateAddress {
            contract: ContentAddress([0; 32]),
            predicate: ContentAddress([0; 32]),
        },
        predicate_data: dec_vars,
        state_mutations: vec![],
    }];
    let access = Access {
        solutions: Arc::new(solutions),
        index: 0,
    };
    let op_gas_cost = &|_: &Op| 1;
    exec_ops(&ops, access, &EmptyState, op_gas_cost, GasLimit::UNLIMITED)
        .map_err(|ExecError(_, e)| e)
        .map(|stack| stack.into())
}

#[test]
fn test_dec_var_slots() {
    let vars = std::iter::repeat(vec![-1, -2, -3, 900])
        .take(12)
        .collect::<Vec<_>>();

    let mut stack = crate::Stack::default();

    super::predicate_data_slots(&mut stack, &vars).unwrap();
    assert_eq!(stack.pop().unwrap(), 12);
}
