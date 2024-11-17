use std::collections::HashSet;

use essential_types::Word;

use crate::{
    error::{OpError, StackError},
    sets::decode_set,
    OpResult, Stack,
};

#[cfg(test)]
mod tests;

/// `Pred::EqRange` implementation.
pub(crate) fn eq_range(stack: &mut Stack) -> OpResult<()> {
    // Pop the length off the stack.
    let len = stack.pop()?;

    // If the length is 0, the ranges are equal.
    if len == 0 {
        stack.push(1)?;
        return Ok(());
    }

    let double = len.checked_mul(2).ok_or(StackError::IndexOutOfBounds)?;
    let len: usize = len.try_into().map_err(|_| StackError::IndexOutOfBounds)?;

    stack.push(double)?;

    let eq = stack.pop_len_words::<_, _, OpError>(|words| {
        let (a, b) = words.split_at(len);
        Ok(a == b)
    })?;

    // Push the result back onto the stack.
    stack.push(eq.into())?;

    Ok(())
}

/// `Pred::EqSet` implementation.
pub(crate) fn eq_set(stack: &mut Stack) -> OpResult<()> {
    let eq = stack.pop_len_words2::<_, _, OpError>(|lhs, rhs| {
        let lhs = decode_set(lhs).collect::<Result<HashSet<&[Word]>, _>>()?;
        let rhs = decode_set(rhs).collect::<Result<HashSet<&[Word]>, _>>()?;
        Ok(lhs == rhs)
    })?;
    stack.push(eq.into())?;
    Ok(())
}
