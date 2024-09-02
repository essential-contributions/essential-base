use super::test_utils::assert_err;
use super::test_utils::assert_stack_ok;
use super::test_utils::ops;
use essential_constraint_asm as asm;
use essential_constraint_asm::Op;
use test_case::test_case;

use crate::sets::decode_set;
use crate::{
    exec_ops, stack,
    test_util::{test_empty_keys, test_solution_data_arr},
};

use super::*;

macro_rules! pub_vars {
    ($($s:expr; $($k:expr => $v:expr),*);* $(;)?) => {
        &[
            $(($s, &[$(($k, $v)),*]),)*
        ]

    };
}

type PubVarRef<'a> = &'a [(SolutionDataIndex, &'a [(&'a [Word], &'a [Word])])];

fn assert_stack_has_set(expected: &[&[Word]]) -> impl Fn(OpResult<Vec<Word>>) {
    let expected = expected
        .iter()
        .map(|s| s.to_vec())
        .collect::<HashSet<Vec<_>>>();
    let total_len = expected.iter().map(|s| s.len() + 1).sum::<usize>();
    move |r| {
        assert!(r.is_ok(), "{:?}", r);
        let mut r = r.unwrap();
        let tl = r.pop().unwrap();
        assert_eq!(tl, total_len as Word);

        let set: HashSet<_> = decode_set(&r)
            .map(Result::unwrap)
            .map(|s| s.to_vec())
            .collect();
        assert_eq!(set, expected);
    }
}

/// These tests give full test coverage for the `pub_var` function.
#[test_case(
    &[0, 3, 99, 2, 0, 2],
    pub_vars![0; &[3, 99] => &[1, 2]] => using assert_stack_ok(&[1, 2])
    ; "Sanity"
)]
#[test_case(
    &[0, 3, 99, 2, 0, 0],
    pub_vars![0; &[3, 99] => &[1, 2]] => using assert_stack_ok(&[])
    ; "empty"
)]
#[test_case(
    &[1, 3, 99, 2, 0, 2],
    pub_vars![0; &[3, 99] => &[1, 2]] => using assert_err!(OpError::Access(AccessError::PathwayOutOfBounds(1)))
    ; "Pathway out of bounds"
)]
#[test_case(
    &[-1, 3, 99, 2, 0, 2],
    pub_vars![0; &[3, 99] => &[1, 2]] => using assert_err!(OpError::Access(AccessError::PathwayOutOfBounds(-1)))
    ; "Negative pathway"
)]
#[test_case(
    &[],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PubVarValueLen)))
    ; "Missing value length"
)]
#[test_case(
    &[100],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PubVarValueIx)))
    ; "Missing value index"
)]
#[test_case(
    &[0, 3, 99, 2, -1, -1],
    pub_vars![0; &[3, 99] => &[1, 2]] => using assert_err!(OpError::Access(AccessError::InvalidAccessRange))
    ; "Negative value index and length"
)]
#[test_case(
    &[100, 100],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PubVarKeyLen)))
    ; "Missing key length"
)]
#[test_case(
    &[-1, 100, 100],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::KeyLengthOutOfBounds(-1)))
    ; "Negative key length"
)]
#[test_case(
    &[0, 3, 99, 4, 100, 100],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PubVarKey)))
    ; "key length too large"
)]
#[test_case(
    &[3, 99, 2, 100, 100],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PubVarPathwayIx)))
    ; "missing pathway_ix"
)]
#[test_case(
    &[Word::MAX, 3, 99, 2, 100, 100],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::PathwayOutOfBounds(Word::MAX)))
    ; "pathway_ix too large"
)]
#[test_case(
    &[0, 3, 100, 2, 100, 100],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::PubVarKeyOutOfBounds))
    ; "key not in map"
)]
#[test_case(
    &[0, 3, 99, 2, 0, 100],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::PubVarDataOutOfBounds))
    ; "value range end out of bounds"
)]
#[test_case(
    &[0, 3, 99, 2, 100, 1],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::PubVarDataOutOfBounds))
    ; "value range start out of bounds"
)]
#[test_case(
    &{
        let mut v = vec![1; Stack::SIZE_LIMIT];
        *v.last_mut().unwrap() = 20;
        v
    },
    pub_vars![1; &[1] => &[42; 21]] =>
    using assert_err!(OpError::Stack(StackError::Overflow))
    ; "values over flow the stack"
)]
fn test_pub_var(words: &[Word], pub_vars: PubVarRef) -> OpResult<Vec<Word>> {
    let mut stack = stack::Stack::default();
    stack.extend(words.to_vec()).unwrap();

    let pub_vars = to_pub_vars(pub_vars);
    pub_var(&mut stack, &pub_vars).map(|_| stack.into())
}

#[test_case(
    &[0, 3, 99, 2],
    pub_vars![0; &[3, 99] => &[1, 2, 9]] => using assert_stack_ok(&[3])
    ; "Sanity"
)]
#[test_case(
    &[1, 3, 99, 2],
    pub_vars![0; &[3, 99] => &[1, 2]] => using assert_err!(OpError::Access(AccessError::PathwayOutOfBounds(1)))
    ; "Pathway out of bounds"
)]
#[test_case(
    &[-1, 3, 99, 2],
    pub_vars![0; &[3, 99] => &[1, 2]] => using assert_err!(OpError::Access(AccessError::PathwayOutOfBounds(-1)))
    ; "Negative pathway"
)]
#[test_case(
    &[],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PubVarKeyLen)))
    ; "Missing key length"
)]
#[test_case(
    &[-1],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::KeyLengthOutOfBounds(-1)))
    ; "Negative key length"
)]
#[test_case(
    &[0, 3, 99, 4],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PubVarKey)))
    ; "key length too large"
)]
#[test_case(
    &[3, 99, 2],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PubVarPathwayIx)))
    ; "missing pathway_ix"
)]
#[test_case(
    &[Word::MAX, 3, 99, 2],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::PathwayOutOfBounds(Word::MAX)))
    ; "pathway_ix too large"
)]
#[test_case(
    &[0, 3, 100, 2],
    pub_vars![0; &[3, 99] => &[1, 2]] =>
    using assert_err!(OpError::Access(AccessError::PubVarKeyOutOfBounds))
    ; "key not in map"
)]
fn test_pub_var_len(words: &[Word], pub_vars: PubVarRef) -> OpResult<Vec<Word>> {
    let mut stack = stack::Stack::default();
    stack.extend(words.to_vec()).unwrap();

    let pub_vars = to_pub_vars(pub_vars);
    pub_var_len(&mut stack, &pub_vars).map(|_| stack.into())
}

#[test_case(
    ops![
        asm::Stack::Push(0),
        asm::Stack::Push(3),
        asm::Stack::Push(99),
        asm::Stack::Push(2),
        asm::Stack::Push(0),
        asm::Stack::Push(2),
        asm::Access::PubVar,
    ],
    pub_vars![0; &[3, 99] => &[1, 2]] => using assert_stack_ok(&[1, 2])
    ; "sanity pub var"
)]
#[test_case(
    ops![
        asm::Stack::Push(5),
        asm::Stack::Push(1),
        asm::Stack::Repeat,
        asm::Access::RepeatCounter,
        asm::Access::RepeatCounter,
        asm::Access::RepeatCounter,
        asm::Stack::Push(2),
        asm::Access::RepeatCounter,
        asm::Stack::Push(5),
        asm::Access::RepeatCounter,
        asm::Alu::Sub,
        asm::Access::PubVar,
        asm::Stack::RepeatEnd,
    ],
    pub_vars![
        0; &[0, 0] => &(0..5).collect::<Vec<_>>();
        1; &[1, 1] => &(0..5).collect::<Vec<_>>();
        2; &[2, 2] => &(0..5).collect::<Vec<_>>();
        3; &[3, 3] => &(0..5).collect::<Vec<_>>();
        4; &[4, 4] => &(0..5).collect::<Vec<_>>();
    ]
    => using assert_stack_ok(&[0, 1, 2, 3, 4, 1, 2, 3, 4, 2, 3, 4, 3, 4, 4])
    ; "multiple keys pub var"
)]
#[test_case(
    ops![
        asm::Stack::Push(0),
        asm::Stack::Push(3),
        asm::Stack::Push(99),
        asm::Stack::Push(2),
        asm::Access::PubVarLen,
    ],
    pub_vars![0; &[3, 99] => &[1, 3]] => using assert_stack_ok(&[2])
    ; "sanity pub var len"
)]
#[test_case(
    ops![
        asm::Stack::Push(5),
        asm::Stack::Push(1),
        asm::Stack::Repeat,
        asm::Access::RepeatCounter,
        asm::Access::RepeatCounter,
        asm::Access::RepeatCounter,
        asm::Stack::Push(2),
        asm::Access::PubVarLen,
        asm::Stack::RepeatEnd,
    ],
    pub_vars![
        0; &[0, 0] => &(0..5).collect::<Vec<_>>();
        1; &[1, 1] => &(0..5).collect::<Vec<_>>();
        2; &[2, 2] => &(0..5).collect::<Vec<_>>();
        3; &[3, 3] => &(0..5).collect::<Vec<_>>();
        4; &[4, 4] => &(0..5).collect::<Vec<_>>();
    ]
    => using assert_stack_ok(&[5, 5, 5, 5, 5])
    ; "multiple keys pub var len"
)]
fn test_pub_var_ops(ops: Vec<Op>, pub_vars: PubVarRef) -> OpResult<Vec<Word>> {
    let pub_vars = to_pub_vars(pub_vars);
    let access = Access {
        solution: SolutionAccess {
            data: test_solution_data_arr(),
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: &pub_vars,
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

#[test_case(
    &[0],
    &[(0, &[&[42]])] => using assert_stack_ok(&[42, 1, 2])
    ; "Sanity"
)]
#[test_case(
    &[20],
    &[
        (0, &[&[42], &[22, 56]]),
        (20, &[&[-1, 88, 67], &[12, -12], &[9999, -88]]),
        (2, &[&[42, 1]])
    ] =>
    using assert_stack_has_set(&[&[-1, 88, 67], &[12, -12], &[9999, -88]])
    ; "multi"
)]
#[test_case(
    &[],
    &[(0, &[&[42]])] =>
    using assert_err!(OpError::Access(AccessError::MissingArg(MissingAccessArgError::PushPubVarKeysPathwayIx)))
    ; "missing pathway_ix"
)]
#[test_case(
    &[Word::MAX],
    &[(0, &[&[42]])] =>
    using assert_err!(OpError::Access(AccessError::PathwayOutOfBounds(Word::MAX)))
    ; "pathway too large"
)]
#[test_case(
    &[-10],
    &[(0, &[&[42]])] =>
    using assert_err!(OpError::Access(AccessError::PathwayOutOfBounds(-10)))
    ; "pathway negative"
)]
#[test_case(
    &[1],
    &[(0, &[&[42]])] =>
    using assert_err!(OpError::Access(AccessError::PathwayOutOfBounds(1)))
    ; "pathway out of bounds"
)]
#[test_case(
    &vec![1; Stack::SIZE_LIMIT],
    &[(1, &[&[42]])] =>
    using assert_err!(OpError::Stack(StackError::Overflow))
    ; "values over flow the stack"
)]
fn test_push_pub_var_keys(
    words: &[Word],
    pub_var_keys: &[(SolutionDataIndex, &[&[Word]])],
) -> OpResult<Vec<Word>> {
    let mut stack = stack::Stack::default();
    stack.extend(words.to_vec()).unwrap();

    let pub_vars = pub_var_keys
        .iter()
        .map(|(s, vars)| {
            let vars = vars.iter().map(|k| (k.to_vec(), vec![1])).collect();
            (*s, vars)
        })
        .collect();
    push_pub_var_keys(&pub_vars, &mut stack).map(|_| stack.into())
}

#[test_case(
    ops![
        asm::Stack::Push(72),
        asm::Access::PubVarKeys,
    ],
    &[(72, &[&[42]])] => using assert_stack_ok(&[42, 1, 2])
    ; "Sanity"
)]
#[test_case(
    ops![
        asm::Stack::Push(1),
        asm::Access::PubVarKeys,
    ],
    &[
        (1, &[&[0, 0], &[1, 1], &[2, 2], &[3, 3], &[4, 4]]),
    ]
    => using assert_stack_has_set(&[&[0, 0], &[1, 1], &[2, 2], &[3, 3], &[4, 4]])
    ; "multiple keys"
)]
fn test_push_pub_var_keys_ops(
    ops: Vec<Op>,
    pub_var_keys: &[(SolutionDataIndex, &[&[Word]])],
) -> OpResult<Vec<Word>> {
    let pub_vars = pub_var_keys
        .iter()
        .map(|(s, vars)| {
            let vars = vars.iter().map(|k| (k.to_vec(), vec![1])).collect();
            (*s, vars)
        })
        .collect();
    let access = Access {
        solution: SolutionAccess {
            data: test_solution_data_arr(),
            index: 0,
            mutable_keys: test_empty_keys(),
            transient_data: &pub_vars,
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

fn to_pub_vars(pub_vars: PubVarRef) -> TransientData {
    pub_vars
        .iter()
        .map(|(s, vars)| {
            let vars = vars.iter().map(|(k, v)| (k.to_vec(), v.to_vec())).collect();
            (*s, vars)
        })
        .collect()
}
