use crate::{
    error::{OpError, StackError},
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
