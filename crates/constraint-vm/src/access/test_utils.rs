use essential_types::Word;

use crate::{error::OpError, OpResult};

macro_rules! ops {
    ($($op:expr),* $(,)?) => {
        vec![$(Op::from($op)),*]
    };
}

pub(super) use ops;

pub(super) fn assert_stack_ok(expected: &[Word]) -> impl Fn(OpResult<Vec<Word>>) {
    let expected = expected.to_vec();
    move |r| assert_eq!(*r.as_ref().unwrap(), expected, "{:?}", r)
}

pub(super) fn assert_err_inner(f: impl Fn(&OpError) -> bool) -> impl Fn(OpResult<Vec<Word>>) {
    move |r| assert!(f(r.as_ref().unwrap_err()), "{:?}", r)
}

macro_rules! assert_err {
    ($pat:pat) => {
        $crate::access::test_utils::assert_err_inner(|e| matches!(e, $pat))
    };
}

pub(super) use assert_err;
