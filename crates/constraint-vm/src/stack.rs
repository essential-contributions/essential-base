//! Stack operation and related stack manipulation implementations.

use crate::{asm::Word, error::StackError, StackResult};

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
        let len = self.pop_len()?;
        let ix = self
            .len()
            .checked_sub(len)
            .ok_or(StackError::IndexOutOfBounds)?;
        let out = f(&self[ix..])?;
        self.0.truncate(ix);
        Ok(out)
    }
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
        let stack = exec_ops(ops.iter().copied(), TEST_ACCESS).unwrap();
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
        let stack = exec_ops(ops.iter().copied(), TEST_ACCESS).unwrap();
        assert_eq!(&stack[..], &[3, 2, 1, 42, 42]);
    }

    #[test]
    fn push1() {
        let ops = &[Stack::Push(42).into()];
        let stack = exec_ops(ops.iter().copied(), TEST_ACCESS).unwrap();
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
        let stack = exec_ops(ops.iter().copied(), TEST_ACCESS).unwrap();
        assert_eq!(&stack[..], &[1, 3]);
    }

    #[test]
    fn pop_empty() {
        let ops = &[Stack::Pop.into()];
        match eval_ops(ops.iter().copied(), TEST_ACCESS) {
            Err(ConstraintError::Op(0, OpError::Stack(StackError::Empty))) => (),
            _ => panic!("expected empty stack error"),
        }
    }

    #[test]
    fn index_oob() {
        let ops = &[Stack::Push(0).into(), Stack::DupFrom.into()];
        match eval_ops(ops.iter().copied(), TEST_ACCESS) {
            Err(ConstraintError::Op(1, OpError::Stack(StackError::IndexOutOfBounds))) => (),
            _ => panic!("expected index out-of-bounds stack error"),
        }
    }
}
