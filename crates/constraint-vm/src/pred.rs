use std::ops::{Range, RangeInclusive};

use essential_types::Word;

use crate::{
    access::{resolve_decision_var, state_slot_range},
    error::{AccessError, StackError},
    Access, OpResult, Stack,
};

#[cfg(test)]
mod tests;

/// `Pred::EqRange` implementation.
pub(crate) fn eq_range(stack: &mut Stack) -> OpResult<()> {
    // Get the ranges off the stack.
    let [start_a, start_b, len] = stack.pop3()?;

    // If the length is 0, the ranges are equal.
    if len == 0 {
        stack.push(1)?;
        return Ok(());
    }

    // Make the ranges.
    let (range_a, range_b) = make_ranges(start_a, start_b, len)?;

    // Get the slices from the stack.
    let slice_a = stack.get(range_a).ok_or(StackError::IndexOutOfBounds)?;
    let slice_b = stack.get(range_b).ok_or(StackError::IndexOutOfBounds)?;

    // Compare the slices.
    let eq = slice_a == slice_b;

    // Push the result back onto the stack.
    stack.push(eq.into())?;

    Ok(())
}

/// `Pred::EqRangeDecVar` implementation.
pub(crate) fn eq_range_dec_var(access: Access, stack: &mut Stack) -> OpResult<()> {
    // Get the ranges off the stack.
    let [start_stack, start_dec_var, len] = stack.pop3()?;

    // If the length is 0, the ranges are equal.
    if len == 0 {
        stack.push(1)?;
        return Ok(());
    }

    // Make the stack range.
    let range_stack = make_range(start_stack, len)?;
    let range_dec_var = make_forward_range(start_dec_var, len)?;

    // Get the slices from the stack.
    let slice_stack = stack.get(range_stack).ok_or(StackError::IndexOutOfBounds)?;

    // TODO: Update this when we introduce transient data and remove it from decision variables.
    let dec_vars: Vec<Word> = range_dec_var
        .map(|v| resolve_decision_var(access.solution.data, access.solution.index, v))
        .collect::<Result<Vec<Word>, AccessError>>()?;

    // Compare the slices.
    let eq = slice_stack == &dec_vars[..];

    // Push the result back onto the stack.
    stack.push(eq.into())?;

    Ok(())
}

/// `Pred::EqRangeState` implementation.
pub(crate) fn eq_range_state(access: Access, stack: &mut Stack) -> OpResult<()> {
    // Get the ranges off the stack.
    let [start_stack, start_state, delta, len] = stack.pop4()?;

    // If the length is 0, the ranges are equal.
    if len == 0 {
        stack.push(1)?;
        return Ok(());
    }

    // Make the stack range.
    let range_stack = make_range(start_stack, len)?;

    // Get the slices from the stack.
    let slice_stack = stack.get(range_stack).ok_or(StackError::IndexOutOfBounds)?;

    let range_state = state_slot_range(access.state_slots, start_state, len, delta)?;

    // Compare the stack to state where None is 0.
    // Return early if a not equal is found.
    for (stack_v, state) in slice_stack.iter().zip(range_state) {
        match state {
            Some(state) => {
                if stack_v != state {
                    stack.push(0)?;
                    return Ok(());
                }
            }
            None => {
                if *stack_v != 0 {
                    stack.push(0)?;
                    return Ok(());
                }
            }
        }
    }

    // Push the true onto the stack.
    stack.push(1)?;

    Ok(())
}

fn make_ranges(
    start_a: Word,
    start_b: Word,
    len: Word,
) -> OpResult<(RangeInclusive<usize>, RangeInclusive<usize>)> {
    let range_a = make_range(start_a, len)?;
    let range_b = make_range(start_b, len)?;

    Ok((range_a, range_b))
}

fn make_range(start: Word, len: Word) -> OpResult<RangeInclusive<usize>> {
    // Convert the ranges to usize.
    let start: usize = start.try_into().map_err(|_| StackError::IndexOutOfBounds)?;
    let len: usize = len.try_into().map_err(|_| StackError::IndexOutOfBounds)?;

    // Calculate the end of the ranges.
    let len = len.checked_sub(1).ok_or(StackError::IndexOutOfBounds)?;
    let end = start.checked_sub(len).ok_or(StackError::IndexOutOfBounds)?;

    Ok(end..=start)
}

fn make_forward_range(start: Word, len: Word) -> OpResult<Range<usize>> {
    // Convert the ranges to usize.
    let start: usize = start.try_into().map_err(|_| StackError::IndexOutOfBounds)?;
    let len: usize = len.try_into().map_err(|_| StackError::IndexOutOfBounds)?;

    // Calculate the end of the ranges.
    let end = start.checked_add(len).ok_or(StackError::IndexOutOfBounds)?;

    Ok(start..end)
}
