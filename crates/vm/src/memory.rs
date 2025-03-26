use essential_types::Word;

use crate::error::MemoryError;

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
    pub fn alloc(&mut self, size: Word) -> Result<(), MemoryError> {
        let size = usize::try_from(size).map_err(|_| MemoryError::Overflow)?;
        let new_size = self
            .0
            .len()
            .checked_add(size)
            .ok_or(MemoryError::Overflow)?;
        if new_size > Self::SIZE_LIMIT {
            return Err(MemoryError::Overflow);
        }
        self.0.resize(new_size, 0);
        Ok(())
    }

    /// Store a word at the given address.
    pub fn store(&mut self, address: Word, value: Word) -> Result<(), MemoryError> {
        let index = usize::try_from(address).map_err(|_| MemoryError::IndexOutOfBounds)?;
        *self.0.get_mut(index).ok_or(MemoryError::IndexOutOfBounds)? = value;
        Ok(())
    }

    /// Load a word from the given address.
    pub fn load(&self, address: Word) -> Result<Word, MemoryError> {
        let index = usize::try_from(address).map_err(|_| MemoryError::IndexOutOfBounds)?;
        Ok(*self.0.get(index).ok_or(MemoryError::IndexOutOfBounds)?)
    }

    /// Store a range of words starting at the given address.
    pub fn store_range(&mut self, address: Word, values: &[Word]) -> Result<(), MemoryError> {
        let address = usize::try_from(address).map_err(|_| MemoryError::IndexOutOfBounds)?;
        let end = address
            .checked_add(values.len())
            .ok_or(MemoryError::Overflow)?;
        if end > self.0.len() {
            return Err(MemoryError::IndexOutOfBounds);
        }
        self.0[address..end].copy_from_slice(values);
        Ok(())
    }

    /// Load a range of words starting at the given address.
    pub fn load_range(&self, address: Word, size: Word) -> Result<Vec<Word>, MemoryError> {
        let address = usize::try_from(address).map_err(|_| MemoryError::IndexOutOfBounds)?;
        let size = usize::try_from(size).map_err(|_| MemoryError::Overflow)?;
        let end = address.checked_add(size).ok_or(MemoryError::Overflow)?;
        if end > self.0.len() {
            return Err(MemoryError::IndexOutOfBounds);
        }
        Ok(self.0[address..end].to_vec())
    }

    /// Truncate memory to the given `new_len`, freeing all memory that follows.
    pub fn free(&mut self, new_len: Word) -> Result<(), MemoryError> {
        let new_len = usize::try_from(new_len).map_err(|_| MemoryError::IndexOutOfBounds)?;
        if new_len > self.0.len() {
            return Err(MemoryError::IndexOutOfBounds);
        }
        self.0.truncate(new_len);
        self.0.shrink_to_fit();
        Ok(())
    }

    /// Current len of the memory.
    pub fn len(&self) -> Result<Word, MemoryError> {
        self.0.len().try_into().map_err(|_| MemoryError::Overflow)
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
    type Error = MemoryError;
    fn try_from(words: Vec<Word>) -> Result<Self, Self::Error> {
        if words.len() > Self::SIZE_LIMIT {
            Err(MemoryError::Overflow)
        } else {
            Ok(Self(words))
        }
    }
}

impl core::ops::Deref for Memory {
    type Target = [Word];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
