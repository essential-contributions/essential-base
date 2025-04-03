use crate::error::{OpError, OpResult};
use essential_types::Word;

/// Assert that the result is ok and that the stack equals the expected value.
pub(super) fn assert_stack_ok<E: std::fmt::Display + std::fmt::Debug>(
    expected: &[Word],
) -> impl Fn(OpResult<Vec<Word>, E>) {
    let expected = expected.to_vec();
    move |r| assert_eq!(*r.as_ref().unwrap(), expected, "{:?}", r)
}

pub(super) fn assert_err_inner<E: std::fmt::Display + std::fmt::Debug>(
    f: impl Fn(&OpError<E>) -> bool,
) -> impl Fn(OpResult<Vec<Word>, E>) {
    move |r| assert!(f(r.as_ref().unwrap_err()), "{:?}", r)
}

/// Helper macro for asserting that the result is the expected error.
macro_rules! assert_err {
    ($pat:pat) => {
        $crate::access::test_utils::assert_err_inner(|e| matches!(e, $pat))
    };
}

pub(super) use assert_err;
