//! State slot operation implementations and related items.

use core::ops::Range;

use essential_constraint_vm::{error::StackError, Stack};

use crate::{asm::Word, OpSyncResult, StateSlotsError, StateSlotsResult};

#[cfg(test)]
mod tests;

/// A type representing the VM's mutable state slots.
///
/// `StateSlots` is a thin wrapper around a `Vec<Vec<Word>>`. The `Vec` mutable methods
/// are predicateionally not exposed in order to maintain close control over capacity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateSlotsMut(Vec<Vec<Word>>);

impl StateSlotsMut {
    /// The maximum number of slots that can be allocated.
    pub const SLOT_LIMIT: usize = 4096;

    /// The maximum number of words stored in a single value.
    pub const VALUE_LIMIT: usize = 4096;

    /// Allocate new slots to the end of the vector.
    pub fn alloc_slots(&mut self, size: usize) -> StateSlotsResult<()> {
        if self.len() + size > Self::SLOT_LIMIT {
            return Err(StateSlotsError::Overflow);
        }
        self.0.resize_with(self.len() + size, Default::default);
        Ok(())
    }

    /// Load a value at the given slot index.
    pub fn load(&self, slot_ix: usize, range: Range<usize>) -> StateSlotsResult<&[Word]> {
        let slot = self
            .get(slot_ix)
            .ok_or(StateSlotsError::IndexOutOfBounds)?
            .get(range)
            .ok_or(StateSlotsError::IndexOutOfBounds)?;
        Ok(slot)
    }

    /// Store the given value at the given slot `index`.
    pub fn store(
        &mut self,
        slot_ix: usize,
        value_ix: usize,
        data: Vec<Word>,
    ) -> StateSlotsResult<()> {
        let slot = self
            .0
            .get_mut(slot_ix)
            .ok_or(StateSlotsError::IndexOutOfBounds)?;

        if slot.len().saturating_add(data.len()) > Self::VALUE_LIMIT {
            return Err(StateSlotsError::Overflow);
        }

        let (_, rem) = slot
            .split_at_mut_checked(value_ix)
            .ok_or(StateSlotsError::IndexOutOfBounds)?;
        let len = rem.len().min(data.len());
        rem[..len].copy_from_slice(&data[..len]);
        if len < data.len() {
            slot.extend_from_slice(&data[len..]);
        }
        Ok(())
    }

    /// Truncate the value at the given slot index.
    pub fn truncate(&mut self, slot_ix: usize, len: usize) -> StateSlotsResult<()> {
        self.0
            .get_mut(slot_ix)
            .ok_or(StateSlotsError::IndexOutOfBounds)?
            .truncate(len);
        Ok(())
    }

    /// Get the length of a value at the given slot index.
    pub fn value_len(&self, slot_ix: usize) -> StateSlotsResult<usize> {
        let slot = self
            .0
            .get(slot_ix)
            .ok_or(StateSlotsError::IndexOutOfBounds)?;
        Ok(slot.len())
    }

    /// Store the given values starting at the given slot `index`.
    pub fn store_slots_range(
        &mut self,
        index: usize,
        values: Vec<Vec<Word>>,
    ) -> StateSlotsResult<()> {
        if values.iter().any(|val| val.len() > Self::VALUE_LIMIT) {
            return Err(StateSlotsError::Overflow);
        }

        let slots = self
            .0
            .get_mut(index..(index + values.len()))
            .ok_or(StateSlotsError::IndexOutOfBounds)?;

        for (slot, value) in slots.iter_mut().zip(values) {
            *slot = value;
        }
        Ok(())
    }
}

impl core::ops::Deref for StateSlotsMut {
    type Target = Vec<Vec<Word>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<StateSlotsMut> for Vec<Vec<Word>> {
    fn from(state_slots: StateSlotsMut) -> Self {
        state_slots.0
    }
}

/// `StateMemory::AllocSlots` operation.
pub fn alloc_slots(stack: &mut Stack, slots: &mut StateSlotsMut) -> OpSyncResult<()> {
    let size = stack.pop()?;
    let size = usize::try_from(size).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    slots.alloc_slots(size)?;
    Ok(())
}

/// `StateMemory::Length` operation.
pub fn length(stack: &mut Stack, slots: &StateSlotsMut) -> OpSyncResult<()> {
    let len = Word::try_from(slots.len()).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    stack.push(len)?;
    Ok(())
}

/// `StateMemory::ValueLen` operation.
pub fn value_len(stack: &mut Stack, slots: &StateSlotsMut) -> OpSyncResult<()> {
    let slot = stack.pop()?;
    let slot = usize::try_from(slot).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let len =
        Word::try_from(slots.value_len(slot)?).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    stack.push(len)?;
    Ok(())
}

/// `StateMemory::Truncate` operation.
pub fn truncate(stack: &mut Stack, slots: &mut StateSlotsMut) -> OpSyncResult<()> {
    let len = stack.pop()?;
    let index = stack.pop()?;
    let len = usize::try_from(len).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let index = usize::try_from(index).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    slots.truncate(index, len)?;
    Ok(())
}

/// `StateMemory::Load` operation.
pub fn load(stack: &mut Stack, slots: &StateSlotsMut) -> OpSyncResult<()> {
    let len = stack.pop()?;
    let value_ix = stack.pop()?;
    let slot_ix = stack.pop()?;
    let slot_ix = usize::try_from(slot_ix).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let range = range_from_start_len(value_ix, len).ok_or(StateSlotsError::IndexOutOfBounds)?;
    let value = slots.load(slot_ix, range)?;
    stack.extend(value.iter().copied())?;
    Ok(())
}

/// `StateMemory::Store` operation.
pub fn store(stack: &mut Stack, slots: &mut StateSlotsMut) -> OpSyncResult<()> {
    let data = stack.pop_len_words::<_, _, StackError>(|value| Ok(value.to_vec()))?;
    let value_ix = stack.pop()?;
    let value_ix = usize::try_from(value_ix).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let slot_ix = stack.pop()?;
    let slot_ix = usize::try_from(slot_ix).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    slots.store(slot_ix, value_ix, data)?;
    Ok(())
}

fn range_from_start_len(start: Word, len: Word) -> Option<std::ops::Range<usize>> {
    let start = usize::try_from(start).ok()?;
    let len = usize::try_from(len).ok()?;
    let end = start.checked_add(len)?;
    Some(start..end)
}
