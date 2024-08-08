use std::collections::HashSet;

use essential_types::Word;

use crate::{
    error::{DecodeError, OpError, StackError},
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
        let lhs = unflatten_keys(lhs).collect::<Result<HashSet<&[Word]>, _>>()?;
        let rhs = unflatten_keys(rhs).collect::<Result<HashSet<&[Word]>, _>>()?;
        Ok(lhs == rhs)
    })?;
    stack.push(eq.into())?;
    Ok(())
}

/// Unflatten the keys, starting from the top of slice.
fn unflatten_keys(words: &[Word]) -> impl '_ + Iterator<Item = OpResult<&[Word]>> {
    let mut ws = words;
    std::iter::from_fn(move || {
        let (len, rest) = ws.split_last()?;
        let ix = match usize::try_from(*len)
            .map_err(|_| StackError::Overflow.into())
            .and_then(|len| {
                rest.len()
                    .checked_sub(len)
                    .ok_or_else(|| DecodeError::Set(words.to_vec()).into())
            }) {
            Ok(ix) => ix,
            Err(e) => return Some(Err(e)),
        };
        let (rest, key) = rest.split_at(ix);
        ws = rest;
        Some(Ok(key))
    })
}
