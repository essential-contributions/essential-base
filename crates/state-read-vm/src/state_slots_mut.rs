//! State slot operation implementations and related items.

use essential_constraint_vm::error::StackError;

use crate::{asm::Word, OpSyncResult, StateSlotsError, StateSlotsResult, Vm};

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
    pub fn load(&self, index: usize) -> StateSlotsResult<&[Word]> {
        let slot = self.get(index).ok_or(StateSlotsError::IndexOutOfBounds)?;
        Ok(slot)
    }

    /// Store the given value at the given slot `index`.
    pub fn store(&mut self, index: usize, value: Vec<Word>) -> StateSlotsResult<()> {
        if value.len() > Self::VALUE_LIMIT {
            return Err(StateSlotsError::Overflow);
        }

        let slot = self
            .0
            .get_mut(index)
            .ok_or(StateSlotsError::IndexOutOfBounds)?;
        *slot = value;
        Ok(())
    }

    /// Store the given values starting at the given slot `index`.
    pub fn store_range(&mut self, index: usize, values: Vec<Vec<Word>>) -> StateSlotsResult<()> {
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

    /// Clear the value at the given slot index.
    pub fn clear(&mut self, index: usize) -> StateSlotsResult<()> {
        self.0
            .get_mut(index)
            .ok_or(StateSlotsError::IndexOutOfBounds)?
            .clear();
        Ok(())
    }

    /// Clear a range of slot values.
    pub fn clear_range(&mut self, range: core::ops::Range<usize>) -> StateSlotsResult<()> {
        self.0
            .get_mut(range)
            .ok_or(StateSlotsError::IndexOutOfBounds)?
            .iter_mut()
            .for_each(|val| val.clear());
        Ok(())
    }

    /// Get the length of a value at the given slot index.
    pub fn value_len(&self, index: usize) -> StateSlotsResult<usize> {
        let slot = self.0.get(index).ok_or(StateSlotsError::IndexOutOfBounds)?;
        Ok(slot.len())
    }

    /// Load a word within a value at the given slot index.
    pub fn load_word(&self, slot: usize, index: usize) -> StateSlotsResult<Word> {
        let word = self
            .get(slot)
            .and_then(|slot| slot.get(index).copied())
            .ok_or(StateSlotsError::IndexOutOfBounds)?;
        Ok(word)
    }

    /// Store a word within a value at the given slot index.
    pub fn store_word(&mut self, slot: usize, index: usize, value: Word) -> StateSlotsResult<()> {
        let word = self
            .0
            .get_mut(slot)
            .and_then(|slot| slot.get_mut(index))
            .ok_or(StateSlotsError::IndexOutOfBounds)?;
        *word = value;
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

/// `StateSlots::AllocSlots` operation.
pub fn alloc_slots(vm: &mut Vm) -> OpSyncResult<()> {
    let size = vm.stack.pop()?;
    let size = usize::try_from(size).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    vm.state_slots_mut.alloc_slots(size)?;
    Ok(())
}

/// `StateSlots::Length` operation.
pub fn length(vm: &mut Vm) -> OpSyncResult<()> {
    let len =
        Word::try_from(vm.state_slots_mut.len()).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    vm.stack.push(len)?;
    Ok(())
}

/// `StateSlots::ValueLen` operation.
pub fn value_len(vm: &mut Vm) -> OpSyncResult<()> {
    let slot = vm.stack.pop()?;
    let slot = usize::try_from(slot).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let len = Word::try_from(vm.state_slots_mut.value_len(slot)?)
        .map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    vm.stack.push(len)?;
    Ok(())
}

/// `StateSlots::Clear` operation.
pub fn clear(vm: &mut Vm) -> OpSyncResult<()> {
    let index = vm.stack.pop()?;
    let index = usize::try_from(index).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    vm.state_slots_mut.clear(index)?;
    Ok(())
}

/// `StateSlots::ClearRange` operation.
pub fn clear_range(vm: &mut Vm) -> OpSyncResult<()> {
    let [index, len] = vm.stack.pop2()?;
    let range = range_from_start_len(index, len).ok_or(StateSlotsError::IndexOutOfBounds)?;
    vm.state_slots_mut.clear_range(range)?;
    Ok(())
}

/// `StateSlots::Load` operation.
pub fn load(vm: &mut Vm) -> OpSyncResult<()> {
    let index = vm.stack.pop()?;
    let index = usize::try_from(index).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let value = vm.state_slots_mut.load(index)?;
    vm.stack.extend(value.iter().copied())?;
    Ok(())
}

/// `StateSlots::Store` operation.
pub fn store(vm: &mut Vm) -> OpSyncResult<()> {
    let index = vm.stack.pop()?;
    let index = usize::try_from(index).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let value = vm
        .stack
        .pop_len_words::<_, _, StackError>(|value| Ok(value.to_vec()))?;
    vm.state_slots_mut.store(index, value)?;
    Ok(())
}

/// `StateSlots::LoadWord` operation.
pub fn load_word(vm: &mut Vm) -> OpSyncResult<()> {
    let [slot, index] = vm.stack.pop2()?;
    let slot = usize::try_from(slot).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let index = usize::try_from(index).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let word = vm.state_slots_mut.load_word(slot, index)?;
    vm.stack.push(word)?;
    Ok(())
}

/// `StateSlots::StoreWord` operation.
pub fn store_word(vm: &mut Vm) -> OpSyncResult<()> {
    let [slot, index, word] = vm.stack.pop3()?;
    let slot = usize::try_from(slot).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    let index = usize::try_from(index).map_err(|_| StateSlotsError::IndexOutOfBounds)?;
    vm.state_slots_mut.store_word(slot, index, word)?;
    Ok(())
}

fn range_from_start_len(start: Word, len: Word) -> Option<std::ops::Range<usize>> {
    let start = usize::try_from(start).ok()?;
    let len = usize::try_from(len).ok()?;
    let end = start.checked_add(len)?;
    Some(start..end)
}
