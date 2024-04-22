use essential_types::Word;

use crate::{error::TemporaryError, OpResult};

#[derive(Default, Debug, PartialEq)]
/// Memory for temporary storage of words.
pub struct Memory(Vec<Word>);

impl Memory {
    /// The maximum number of words that can be stored in memory.
    pub const SIZE_LIMIT: usize = 1024 * 10;

    /// Create a new temporary memory instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a word at the given address.
    pub fn store(&mut self, address: Word, value: Word) -> OpResult<()> {
        let index = usize::try_from(address).map_err(|_| TemporaryError::IndexOutOfBounds)?;
        *self
            .0
            .get_mut(index)
            .ok_or(TemporaryError::IndexOutOfBounds)? = value;
        Ok(())
    }

    /// Load a word from the given address.
    pub fn load(&mut self, address: Word) -> OpResult<Word> {
        let index = usize::try_from(address).map_err(|_| TemporaryError::IndexOutOfBounds)?;
        Ok(*self.0.get(index).ok_or(TemporaryError::IndexOutOfBounds)?)
    }

    /// Push a word onto the memory.
    pub fn push(&mut self, value: Word) -> OpResult<()> {
        if self.0.len() >= Self::SIZE_LIMIT {
            return Err(TemporaryError::Overflow.into());
        }
        self.0.push(value);
        Ok(())
    }

    /// Pop a word from the memory.
    pub fn pop(&mut self) -> OpResult<()> {
        let i = self.0.pop();
        if i.is_none() {
            Err(TemporaryError::Empty.into())
        } else {
            Ok(())
        }
    }
}
