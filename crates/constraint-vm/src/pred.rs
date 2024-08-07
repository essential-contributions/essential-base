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
    // Pop the length off the stack.
    let rhs_len = stack.pop()?;

    // If the length is 0, the sets are both empty set and are equal.
    if rhs_len == 0 {
        let lhs_len = stack.pop()?;
        if lhs_len == 0 {
            stack.push(1.into())?;
        } else {
            stack.push(lhs_len)?;
            stack.pop_len_discard()?;
            stack.push(0.into())?;
        }
        return Ok(());
    }

    let rhs_len: usize = rhs_len
        .try_into()
        .map_err(|_| StackError::IndexOutOfBounds)?;

    // Check the lens are equal.
    let lhs_len_ix = rhs_len
        .checked_add(1)
        .and_then(|i| stack.len().checked_sub(i))
        .ok_or(StackError::IndexOutOfBounds)?;
    let lhs_len = stack.get(lhs_len_ix).ok_or(StackError::IndexOutOfBounds)?;
    let lhs_len: usize = (*lhs_len)
        .try_into()
        .map_err(|_| StackError::IndexOutOfBounds)?;

    let full_len: Word = rhs_len
        .checked_add(1)
        .and_then(|i| lhs_len.checked_add(i))
        .ok_or(StackError::IndexOutOfBounds)?
        .try_into()
        .map_err(|_| StackError::IndexOutOfBounds)?;
    let not_eq_sizes = lhs_len != rhs_len;

    stack.push(full_len)?;

    let eq = stack.pop_len_words::<_, _, OpError>(|words| {
        if not_eq_sizes {
            return Ok(false);
        }
        let (lhs, rhs) = words.split_at(rhs_len);
        // Discard lhs len from start of rhs.
        let rhs = rhs.get(1..).ok_or(StackError::IndexOutOfBounds)?;
        let eq = decode_set(lhs)? == decode_set(rhs)?;

        Ok(eq)
    })?;

    // Push the result back onto the stack.
    stack.push(eq.into())?;

    Ok(())
}

/// Decode a set from a slice of words.
/// The slice is in order from bottom to top of stack.
fn decode_set(mut set: &[Word]) -> OpResult<HashSet<&[Word]>> {
    let s = set;
    let mut out = HashSet::new();
    while let Some((&len, rest)) = set.split_last() {
        let len: usize = len.try_into().map_err(|_| StackError::Overflow)?;
        let ix = rest
            .len()
            .checked_sub(len)
            .ok_or_else(|| DecodeError::Set(s.to_vec()))?;
        let (rest, elem) = rest.split_at(ix);
        out.insert(elem);
        set = rest;
    }
    Ok(out)
}
