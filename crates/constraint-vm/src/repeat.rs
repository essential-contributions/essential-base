use essential_types::Word;

use crate::error::{RepeatError, RepeatResult};

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
    pub repeat_index: usize,
}

impl Repeat {
    /// Create a new repeat stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new repeat location and counter to the stack.
    pub fn repeat_from(&mut self, location: usize, amount: Word) -> RepeatResult<()> {
        if self.stack.len() >= super::Stack::SIZE_LIMIT {
            return Err(RepeatError::Overflow);
        }
        self.stack.push(Slot {
            counter: amount,
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
        let counter = self.counter().map_err(|_| RepeatError::Empty)?;
        if counter <= 1 {
            self.stack.pop();
            Ok(None)
        } else {
            let slot = self
                .stack
                .last_mut()
                .expect("Safe because of counter check");
            slot.counter -= 1;
            Ok(Some(slot.repeat_index))
        }
    }
}
