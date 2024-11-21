use essential_types::{convert::bool_from_word, Word};

use crate::{
    error::{OpSyncResult, RepeatError, RepeatResult, StackError},
    Stack,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Default, PartialEq)]
/// A stack of repeat counters.
pub struct Repeat {
    stack: Vec<Slot>,
}

#[derive(Debug, PartialEq)]
struct Slot {
    pub counter: Word,
    pub limit: Direction,
    pub repeat_index: usize,
}

#[derive(Debug, PartialEq)]
enum Direction {
    Up(Word),
    Down,
}

/// `Stack::Repeat` implementation.
pub(crate) fn repeat(pc: usize, stack: &mut Stack, repeat: &mut Repeat) -> OpSyncResult<()> {
    let [num_repeats, count_up] = stack.pop2()?;
    let count_up = bool_from_word(count_up).ok_or(RepeatError::InvalidCountDirection)?;
    let pc = pc.checked_add(1).ok_or(StackError::IndexOutOfBounds)?;
    if count_up {
        repeat.repeat_to(pc, num_repeats)?;
    } else {
        repeat.repeat_from(pc, num_repeats)?;
    }
    Ok(())
}

impl Repeat {
    /// Create a new repeat stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new repeat location and counter to the stack.
    /// Counts down to 0.
    pub fn repeat_from(&mut self, location: usize, amount: Word) -> RepeatResult<()> {
        if self.stack.len() >= super::Stack::SIZE_LIMIT {
            return Err(RepeatError::Overflow);
        }
        self.stack.push(Slot {
            counter: amount,
            limit: Direction::Down,
            repeat_index: location,
        });
        Ok(())
    }

    /// Add a new repeat location and counter to the stack.
    /// Counts up from 0 to limit - 1.
    pub fn repeat_to(&mut self, location: usize, limit: Word) -> RepeatResult<()> {
        if self.stack.len() >= super::Stack::SIZE_LIMIT {
            return Err(RepeatError::Overflow);
        }
        self.stack.push(Slot {
            counter: 0,
            limit: Direction::Up(limit),
            repeat_index: location,
        });
        Ok(())
    }

    /// Get the current repeat counter.
    ///
    /// Returns an error if the stack is empty.
    pub fn counter(&self) -> RepeatResult<Word> {
        self.stack
            .last()
            .map(|s| s.counter)
            .ok_or(RepeatError::NoCounter)
    }

    // TODO: Update this comment.
    /// If there is a counter on the stack and the counter
    /// has greater then 1 repeat left then this will decrement
    /// the counter and return the index to repeat to.
    ///
    /// If the counter is 1 then this will pop the counter and
    /// return None because the repeat is done.
    ///
    /// If called when the stack is empty then this will return
    /// an error.
    ///
    /// Note that because the code has run once before the
    /// `RepeatEnd` is hit then we stop at 1.
    pub fn repeat(&mut self) -> RepeatResult<Option<usize>> {
        let slot = self.stack.last_mut().ok_or(RepeatError::Empty)?;
        match slot.limit {
            Direction::Up(limit) => {
                if slot.counter >= limit.saturating_sub(1) {
                    self.stack.pop();
                    Ok(None)
                } else {
                    slot.counter += 1;
                    Ok(Some(slot.repeat_index))
                }
            }
            Direction::Down => {
                if slot.counter <= 1 {
                    self.stack.pop();
                    Ok(None)
                } else {
                    slot.counter -= 1;
                    Ok(Some(slot.repeat_index))
                }
            }
        }
    }
}
