use essential_types::Word;

use crate::error::{ConstraintError, ConstraintResult};

/// Helper macro for creating a vector of `Constraint`s.
macro_rules! ops {
    ($($op:expr),* $(,)?) => {
        vec![$(crate::asm::Constraint::from($op)),*]
    };
}

pub(super) use ops;

/// Assert that the result is ok and that the stack equals the expected value.
pub(super) fn assert_stack_ok(expected: &[Word]) -> impl Fn(ConstraintResult<Vec<Word>>) {
    let expected = expected.to_vec();
    move |r| assert_eq!(*r.as_ref().unwrap(), expected, "{:?}", r)
}

pub(super) fn assert_err_inner(
    f: impl Fn(&ConstraintError) -> bool,
) -> impl Fn(ConstraintResult<Vec<Word>>) {
    move |r| assert!(f(r.as_ref().unwrap_err()), "{:?}", r)
}

/// Helper macro for asserting that the result is the expected error.
macro_rules! assert_err {
    ($pat:pat) => {
        $crate::access::test_utils::assert_err_inner(|e| matches!(e, $pat))
    };
}

pub(super) use assert_err;
