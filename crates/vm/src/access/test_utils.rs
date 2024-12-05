use essential_types::Word;

use crate::error::{OpSyncError, OpSyncResult};

/// Assert that the result is ok and that the stack equals the expected value.
pub(super) fn assert_stack_ok(expected: &[Word]) -> impl Fn(OpSyncResult<Vec<Word>>) {
    let expected = expected.to_vec();
    move |r| assert_eq!(*r.as_ref().unwrap(), expected, "{:?}", r)
}

pub(super) fn assert_err_inner(
    f: impl Fn(&OpSyncError) -> bool,
) -> impl Fn(OpSyncResult<Vec<Word>>) {
    move |r| assert!(f(r.as_ref().unwrap_err()), "{:?}", r)
}

/// Helper macro for asserting that the result is the expected error.
macro_rules! assert_err {
    ($pat:pat) => {
        $crate::access::test_utils::assert_err_inner(|e| matches!(e, $pat))
    };
}

pub(super) use assert_err;
