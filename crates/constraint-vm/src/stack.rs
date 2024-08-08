//! Stack operation and related stack manipulation implementations.

use crate::{asm::Word, error::StackError, StackResult};
use essential_types::convert::bool_from_word;

/// The VM's `Stack`, i.e. a `Vec` of `Word`s updated during each step of execution.
///
/// A light wrapper around `Vec<Word>` providing helper methods specific to
/// essential VM execution.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Stack(Vec<Word>);

impl Stack {
    /// Limit the stack size to 32KB to avoid memory bloat during parallel constraint checking.
    pub const SIZE_LIMIT: usize = 4096;

    /// Push a word to the stack.
    ///
    /// Errors in the case that pushing an element would cause the stack to overflow.
    pub fn push(&mut self, word: Word) -> StackResult<()> {
        if self.len() >= Self::SIZE_LIMIT {
            return Err(StackError::Overflow);
        }
        self.0.push(word);
        Ok(())
    }

    /// Extend the stack by with the given iterator yielding words.
    ///
    /// Errors in the case that pushing an element would cause the stack to overflow.
    pub fn extend(&mut self, words: impl IntoIterator<Item = Word>) -> StackResult<()> {
        for word in words {
            self.push(word)?;
        }
        Ok(())
    }

    /// The DupFrom op implementation.
    pub(crate) fn dup_from(&mut self) -> StackResult<()> {
        let rev_ix_w = self.pop()?;
        let rev_ix = usize::try_from(rev_ix_w).map_err(|_| StackError::IndexOutOfBounds)?;
        let ix = self
            .len()
            .checked_sub(rev_ix)
            .and_then(|i| i.checked_sub(1))
            .ok_or(StackError::IndexOutOfBounds)?;
        let w = *self.get(ix).ok_or(StackError::IndexOutOfBounds)?;
        self.push(w)?;
        Ok(())
    }

    /// The SwapIndex op implementation.
    pub(crate) fn swap_index(&mut self) -> StackResult<()> {
        let rev_ix_w = self.pop()?;
        let top_ix = self
            .len()
            .checked_sub(1)
            .ok_or(StackError::IndexOutOfBounds)?;
        let rev_ix = usize::try_from(rev_ix_w).map_err(|_| StackError::IndexOutOfBounds)?;
        let ix = top_ix
            .checked_sub(rev_ix)
            .ok_or(StackError::IndexOutOfBounds)?;
        self.0.swap(ix, top_ix);
        Ok(())
    }

    /// The Select op implementation.
    pub(crate) fn select(&mut self) -> StackResult<()> {
        self.pop().and_then(|cond_w| {
            self.pop2_push1(|w0, w1| {
                Ok(
                    if bool_from_word(cond_w).ok_or(StackError::InvalidCondition(cond_w))? {
                        w1
                    } else {
                        w0
                    },
                )
            })
        })?;
        Ok(())
    }

    /// The SelectRange op implementation.
    pub(crate) fn select_range(&mut self) -> StackResult<()> {
        let cond_w = self.pop()?;
        let cond = bool_from_word(cond_w).ok_or(StackError::InvalidCondition(cond_w))?;
        let len = self.pop_len()?;
        if len == 0 {
            return Ok(());
        }
        // check that `len` is at most half the stack length
        self.len()
            .checked_sub(len.checked_mul(2).ok_or(StackError::IndexOutOfBounds)?)
            .ok_or(StackError::IndexOutOfBounds)?;
        // stack: [arr_a_0, ..arr_a_N, arr_b_0, ..arr_b_N]
        let arr_b_index = self.len() - len;
        if cond {
            // copy arr_b to the space arr_a holds
            let arr_a_index = arr_b_index - len;
            self.0
                .copy_within(arr_b_index..(arr_b_index + len), arr_a_index);
        }
        // pop the topmost range that is arr_b
        self.0.truncate(arr_b_index);
        Ok(())
    }

    /// A wrapper around `Vec::pop`, producing an error in the case that the stack is empty.
    pub fn pop(&mut self) -> StackResult<Word> {
        self.0.pop().ok_or(StackError::Empty)
    }

    /// Pop the top 2 values from the stack.
    ///
    /// The last values popped appear first in the returned fixed-size array.
    pub fn pop2(&mut self) -> StackResult<[Word; 2]> {
        let w1 = self.pop()?;
        let w0 = self.pop()?;
        Ok([w0, w1])
    }

    /// Pop the top 3 values from the stack.
    ///
    /// The last values popped appear first in the returned fixed-size array.
    pub fn pop3(&mut self) -> StackResult<[Word; 3]> {
        let w2 = self.pop()?;
        let [w0, w1] = self.pop2()?;
        Ok([w0, w1, w2])
    }

    /// Pop the top 4 values from the stack.
    ///
    /// The last values popped appear first in the returned fixed-size array.
    pub fn pop4(&mut self) -> StackResult<[Word; 4]> {
        let w3 = self.pop()?;
        let [w0, w1, w2] = self.pop3()?;
        Ok([w0, w1, w2, w3])
    }

    /// Pop the top 8 values from the stack.
    ///
    /// The last values popped appear first in the returned fixed-size array.
    pub fn pop8(&mut self) -> StackResult<[Word; 8]> {
        let [w4, w5, w6, w7] = self.pop4()?;
        let [w0, w1, w2, w3] = self.pop4()?;
        Ok([w0, w1, w2, w3, w4, w5, w6, w7])
    }

    /// Pop 1 word from the stack, apply the given function and push the returned word.
    pub fn pop1_push1<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(Word) -> Result<Word, E>,
        E: From<StackError>,
    {
        let w = self.pop()?;
        let x = f(w)?;
        self.push(x)?;
        Ok(())
    }

    /// Pop 2 words from the stack, apply the given function and push the returned word.
    pub fn pop2_push1<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(Word, Word) -> Result<Word, E>,
        E: From<StackError>,
    {
        let [w0, w1] = self.pop2()?;
        let x = f(w0, w1)?;
        self.push(x)?;
        Ok(())
    }

    /// Pop 8 words from the stack, apply the given function and push the returned word.
    pub fn pop8_push1<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce([Word; 8]) -> Result<Word, E>,
        E: From<StackError>,
    {
        let ws = self.pop8()?;
        let x = f(ws)?;
        self.push(x)?;
        Ok(())
    }

    /// Pop 1 word from the stack, apply the given function and push the 2 returned words.
    pub fn pop1_push2<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(Word) -> Result<[Word; 2], E>,
        E: From<StackError>,
    {
        let w = self.pop()?;
        let xs = f(w)?;
        self.extend(xs)?;
        Ok(())
    }

    /// Pop 2 words from the stack, apply the given function and push the 2 returned words.
    pub fn pop2_push2<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(Word, Word) -> Result<[Word; 2], E>,
        E: From<StackError>,
    {
        let [w0, w1] = self.pop2()?;
        let xs = f(w0, w1)?;
        self.extend(xs)?;
        Ok(())
    }

    /// Pop 2 words from the stack, apply the given function and push the 4 returned words.
    pub fn pop2_push4<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(Word, Word) -> Result<[Word; 4], E>,
        E: From<StackError>,
    {
        let [w0, w1] = self.pop2()?;
        let xs = f(w0, w1)?;
        self.extend(xs)?;
        Ok(())
    }

    /// Pop a length value from the top of the stack and return it.
    pub fn pop_len(&mut self) -> StackResult<usize> {
        let len_word = self.pop()?;
        let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
        Ok(len)
    }

    /// Pop the length from the top of the stack, then pop and provide that many
    /// words to the given function.
    pub fn pop_len_words<F, O, E>(&mut self, f: F) -> Result<O, E>
    where
        F: FnOnce(&[Word]) -> Result<O, E>,
        E: From<StackError>,
    {
        let (rest, slice) = slice_split_len_words(self).ok_or(StackError::IndexOutOfBounds)?;
        let out = f(slice)?;
        self.0.truncate(rest.len());
        Ok(out)
    }

    /// Pop two slices from the top of the stack, each followed by one word
    /// describing their length, and pass them to the given function.
    /// The top slice is provided to the rhs, the bottom slice is provided to the lhs.
    pub fn pop_len_words2<F, O, E>(&mut self, f: F) -> Result<O, E>
    where
        F: FnOnce(&[Word], &[Word]) -> Result<O, E>,
        E: From<StackError>,
    {
        let (rest, rhs) = slice_split_len_words(self).ok_or(StackError::IndexOutOfBounds)?;
        let (rest, lhs) = slice_split_len_words(rest).ok_or(StackError::IndexOutOfBounds)?;
        let out = f(lhs, rhs)?;
        self.0.truncate(rest.len());
        Ok(out)
    }

    /// Reserve additional capacity for the stack.
    /// Noop if capacity already exists.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }
}

/// Split a length from the top of the stack slice, then split off a slice of
/// that length.
///
/// Returns `Some((remaining, slice_of_len))`.
///
/// Returns `None` if the slice is empty, or the length is greater than the rest
/// of the slice.
fn slice_split_len_words(slice: &[Word]) -> Option<(&[Word], &[Word])> {
    let (len, rest) = slice.split_last()?;
    let len = usize::try_from(*len).ok()?;
    let ix = rest.len().checked_sub(len)?;
    Some(rest.split_at(ix))
}

impl From<Stack> for Vec<Word> {
    fn from(stack: Stack) -> Self {
        stack.0
    }
}

impl From<Vec<Word>> for Stack {
    fn from(vec: Vec<Word>) -> Self {
        Self(vec)
    }
}

impl core::ops::Deref for Stack {
    type Target = Vec<Word>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        asm::Stack,
        error::{ConstraintError, OpError, StackError},
        eval_ops, exec_ops,
        test_util::*,
    };

    #[test]
    fn dup_from_1() {
        let ops = &[
            Stack::Push(42).into(),
            Stack::Push(2).into(),
            Stack::Push(1).into(),
            Stack::Push(0).into(),
            Stack::Push(3).into(), // Index `3` should be the `42` value.
            Stack::DupFrom.into(),
        ];
        let stack = exec_ops(ops, *test_access()).unwrap();
        assert_eq!(&stack[..], &[42, 2, 1, 0, 42]);
    }

    #[test]
    fn dup_from_2() {
        let ops = &[
            Stack::Push(3).into(),
            Stack::Push(2).into(),
            Stack::Push(1).into(),
            Stack::Push(42).into(),
            Stack::Push(0).into(), // Index `0` should be the `42` value.
            Stack::DupFrom.into(),
        ];
        let stack = exec_ops(ops, *test_access()).unwrap();
        assert_eq!(&stack[..], &[3, 2, 1, 42, 42]);
    }

    #[test]
    fn push1() {
        let ops = &[Stack::Push(42).into()];
        let stack = exec_ops(ops, *test_access()).unwrap();
        assert_eq!(&stack[..], &[42]);
    }

    #[test]
    fn push2_pop_push() {
        let ops = &[
            Stack::Push(1).into(),
            Stack::Push(2).into(),
            Stack::Pop.into(),
            Stack::Push(3).into(),
        ];
        let stack = exec_ops(ops, *test_access()).unwrap();
        assert_eq!(&stack[..], &[1, 3]);
    }

    #[test]
    fn pop_empty() {
        let ops = &[Stack::Pop.into()];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(0, OpError::Stack(StackError::Empty))) => (),
            _ => panic!("expected empty stack error"),
        }
    }

    #[test]
    fn index_oob() {
        let ops = &[Stack::Push(0).into(), Stack::DupFrom.into()];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(1, OpError::Stack(StackError::IndexOutOfBounds))) => (),
            _ => panic!("expected index out-of-bounds stack error"),
        }
    }

    #[test]
    fn swap_index() {
        let ops = &[
            Stack::Push(3).into(),
            Stack::Push(4).into(),
            Stack::Push(5).into(),
            Stack::Push(42).into(),
            Stack::Push(2).into(), // Index `2` should be swapped with the `42` value.
            Stack::SwapIndex.into(),
        ];
        let stack = exec_ops(ops, *test_access()).unwrap();
        assert_eq!(&stack[..], &[3, 42, 5, 4]);
    }

    #[test]
    fn swap_index_oob() {
        let ops = &[
            Stack::Push(3).into(),
            Stack::Push(4).into(),
            Stack::Push(2).into(), // Index `2` is out of range.
            Stack::SwapIndex.into(),
        ];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(3, OpError::Stack(StackError::IndexOutOfBounds))) => (),
            _ => panic!("expected index out-of-bounds stack error"),
        }
    }

    #[test]
    fn select() {
        let ops = &[
            Stack::Push(3).into(),
            Stack::Push(4).into(),
            Stack::Push(1).into(),
            Stack::Select.into(),
        ];
        let stack = exec_ops(ops, *test_access()).unwrap();
        assert_eq!(&stack[..], &[4]);
    }

    #[test]
    fn select_range_cond_1() {
        let ops = &[
            Stack::Push(4).into(),
            Stack::Push(4).into(),
            Stack::Push(4).into(),
            Stack::Push(5).into(),
            Stack::Push(5).into(),
            Stack::Push(5).into(),
            Stack::Push(3).into(), // len
            Stack::Push(1).into(), // cond
            Stack::SelectRange.into(),
        ];
        let stack = exec_ops(ops, *test_access()).unwrap();
        assert_eq!(&stack[..], &[5, 5, 5]);
    }

    #[test]
    fn select_range_cond_0() {
        let ops = &[
            Stack::Push(4).into(),
            Stack::Push(4).into(),
            Stack::Push(4).into(),
            Stack::Push(5).into(),
            Stack::Push(5).into(),
            Stack::Push(5).into(),
            Stack::Push(3).into(), // len
            Stack::Push(0).into(), // cond
            Stack::SelectRange.into(),
        ];
        let stack = exec_ops(ops, *test_access()).unwrap();
        assert_eq!(&stack[..], &[4, 4, 4]);
    }

    #[test]
    fn select_range_cond_invalid() {
        let ops = &[
            Stack::Push(4).into(),
            Stack::Push(5).into(),
            Stack::Push(1).into(),  // len
            Stack::Push(42).into(), // cond
            Stack::SelectRange.into(),
        ];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(4, OpError::Stack(StackError::InvalidCondition(42)))) => (),
            _ => panic!("expected invalid condition stack error"),
        }
    }

    #[test]
    fn select_range_len_0() {
        let ops = &[
            Stack::Push(4).into(),
            Stack::Push(5).into(),
            Stack::Push(0).into(), // len
            Stack::Push(0).into(), // cond
            Stack::SelectRange.into(),
        ];
        let stack = exec_ops(ops, *test_access()).unwrap();
        assert_eq!(&stack[..], &[4, 5]);
    }

    #[test]
    fn select_range_len_negative() {
        let ops = &[
            Stack::Push(-42).into(), // len
            Stack::Push(0).into(),   // cond
            Stack::SelectRange.into(),
        ];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(2, OpError::Stack(StackError::IndexOutOfBounds))) => (),
            _ => panic!("expected index out of bounds stack error"),
        }
    }

    #[test]
    fn select_range_len_too_big() {
        let ops = &[
            Stack::Push(4).into(),
            Stack::Push(4).into(),
            Stack::Push(5).into(),
            Stack::Push(2).into(), // len
            Stack::Push(0).into(), // cond
            Stack::SelectRange.into(),
        ];
        match eval_ops(ops, *test_access()) {
            Err(ConstraintError::Op(5, OpError::Stack(StackError::IndexOutOfBounds))) => (),
            _ => panic!("expected index out of bounds stack error"),
        }
    }
}
