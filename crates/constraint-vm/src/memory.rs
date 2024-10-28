use essential_types::Word;

use crate::{error::TemporaryError, OpResult};

#[cfg(test)]
mod tests;

#[derive(Clone, Default, Debug, PartialEq)]
/// Memory for temporary storage of words.
pub struct Memory(Vec<Word>);

impl Memory {
    /// The maximum number of words that can be stored in memory.
    pub const SIZE_LIMIT: usize = 1024 * 10;

    /// Create a new temporary memory instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate more memory to the end of this memory.
    pub fn alloc(&mut self, size: Word) -> OpResult<()> {
        let size = usize::try_from(size).map_err(|_| TemporaryError::Overflow)?;
        let new_size = self
            .0
            .len()
            .checked_add(size)
            .ok_or(TemporaryError::Overflow)?;
        if new_size > Self::SIZE_LIMIT {
            return Err(TemporaryError::Overflow.into());
        }
        self.0.resize(new_size, 0);
        Ok(())
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

    /// Current len of the memory.
    pub fn len(&self) -> OpResult<Word> {
        Ok(self
            .0
            .len()
            .try_into()
            .map_err(|_| TemporaryError::Overflow)?)
    }

    /// Is the memory empty?
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Memory> for Vec<Word> {
    fn from(m: Memory) -> Vec<Word> {
        m.0
    }
}

impl TryFrom<Vec<Word>> for Memory {
    type Error = TemporaryError;
    fn try_from(words: Vec<Word>) -> Result<Self, Self::Error> {
        if words.len() > Self::SIZE_LIMIT {
            Err(TemporaryError::Overflow)
        } else {
            Ok(Self(words))
        }
    }
}
