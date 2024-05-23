//! Access operation implementations.

use crate::{
    error::{AccessError, StackError},
    repeat::Repeat,
    types::convert::bool_from_word,
    OpResult, Stack,
};
use essential_constraint_asm::Word;
use essential_types::{
    convert::word_4_from_u8_32,
    solution::{Solution, SolutionData, SolutionDataIndex},
    Key,
};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests;

/// Transient data map.
pub type TransientData = HashMap<SolutionDataIndex, HashMap<Key, Vec<Word>>>;

/// All necessary solution data and state access required to check an individual intent.
#[derive(Clone, Copy, Debug)]
pub struct Access<'a> {
    /// All necessary solution data access required to check an individual intent.
    pub solution: SolutionAccess<'a>,
    /// The pre and post mutation state slot values for the intent being solved.
    pub state_slots: StateSlots<'a>,
}

/// All necessary solution data access required to check an individual intent.
#[derive(Clone, Copy, Debug)]
pub struct SolutionAccess<'a> {
    /// The input data for each intent being solved within the solution.
    ///
    /// We require *all* intent solution data in order to handle transient
    /// decision variable access.
    pub data: &'a [SolutionData],
    /// Checking is performed for one intent at a time. This index refers to
    /// the checked intent's associated solution data within `data`.
    pub index: usize,
    /// The keys being proposed for mutation for the intent.
    pub mutable_keys: &'a HashSet<&'a [Word]>,
    /// The transient data that points to the data in another solution data index.
    pub transient_data: &'a TransientData,
}

/// The pre and post mutation state slot values for the intent being solved.
#[derive(Clone, Copy, Debug)]
pub struct StateSlots<'a> {
    /// Intent state slot values before the solution's mutations are applied.
    pub pre: &'a StateSlotSlice,
    /// Intent state slot values after the solution's mutations are applied.
    pub post: &'a StateSlotSlice,
}

/// The state slots declared within the intent.
pub type StateSlotSlice = [Vec<Word>];

impl<'a> SolutionAccess<'a> {
    /// A shorthand for constructing a `SolutionAccess` instance for checking
    /// the intent at the given index within the given solution.
    ///
    /// This constructor assumes that the given mutable keys set is correct
    /// for this solution. It is not checked by this function for performance.
    pub fn new(
        solution: &'a Solution,
        intent_index: SolutionDataIndex,
        mutable_keys: &'a HashSet<&[Word]>,
        transient_data: &'a TransientData,
    ) -> Self {
        Self {
            data: &solution.data,
            index: intent_index.into(),
            mutable_keys,
            transient_data,
        }
    }

    /// The solution data associated with the intent currently being checked.
    ///
    /// **Panics** in the case that `self.index` is out of range of the `self.data` slice.
    pub fn this_data(&self) -> &SolutionData {
        self.data
            .get(self.index)
            .expect("intent index out of range of solution data")
    }

    /// The transient data associated with the intent currently being checked.
    pub fn this_transient_data(&self) -> Option<&HashMap<Key, Vec<Word>>> {
        self.transient_data.get(&(self.index as SolutionDataIndex))
    }
}

impl<'a> StateSlots<'a> {
    /// Empty state slots.
    pub const EMPTY: Self = Self {
        pre: &[],
        post: &[],
    };
}

/// A helper for collecting all mutable keys that are proposed for mutation for
/// the intent at the given index.
///
/// Specifically, assists in calculating the `mut_keys_len` for
/// `SolutionAccess`, as this is equal to the `.count()` of the returned iterator.
///
/// **Note:** In the case that the given solution is invalid and contains multiple
/// mutations to the same key, the same key will be yielded multiple times.
pub fn mut_keys(
    solution: &Solution,
    intent_index: SolutionDataIndex,
) -> impl Iterator<Item = &Key> {
    solution
        .state_mutations
        .iter()
        .filter(move |state_mutation| state_mutation.pathway == intent_index)
        .flat_map(|state_mutation| state_mutation.mutations.iter().map(|m| &m.key))
}

/// Get the mutable keys as slices
pub fn mut_keys_slices(
    solution: &Solution,
    intent_index: SolutionDataIndex,
) -> impl Iterator<Item = &[Word]> {
    solution
        .state_mutations
        .iter()
        .filter(move |state_mutation| state_mutation.pathway == intent_index)
        .flat_map(|state_mutation| state_mutation.mutations.iter().map(|m| m.key.as_ref()))
}

/// Get the set of mutable keys for this intent.
pub fn mut_keys_set(solution: &Solution, intent_index: SolutionDataIndex) -> HashSet<&[Word]> {
    mut_keys_slices(solution, intent_index).collect()
}

/// Create a transient data map from the solution.
pub fn transient_data(solution: &Solution) -> TransientData {
    let mut transient_data = HashMap::new();
    for mutation in &solution.transient_data {
        let entry: &mut HashMap<Key, Vec<Word>> =
            transient_data.entry(mutation.pathway).or_default();
        for mutation in &mutation.mutations {
            entry.insert(mutation.key.clone(), mutation.value.clone());
        }
    }
    transient_data
}

/// `Access::DecisionVar` implementation.
pub(crate) fn decision_var(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    stack.pop1_push1(|slot| {
        let ix = usize::try_from(slot).map_err(|_| AccessError::DecisionSlotOutOfBounds)?;
        let w = resolve_decision_var(solution.data, solution.index, ix)?;
        Ok(w)
    })
}

/// `Access::DecisionVarRange` implementation.
pub(crate) fn decision_var_range(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    let [slot, len] = stack.pop2()?;
    let range = range_from_start_len(slot, len).ok_or(AccessError::DecisionSlotOutOfBounds)?;
    for dec_var_ix in range {
        let w = resolve_decision_var(solution.data, solution.index, dec_var_ix)?;
        stack.push(w)?;
    }
    Ok(())
}

/// `Access::MutKeysLen` implementation.
pub(crate) fn mut_keys_len(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    stack.push(
        solution
            .mutable_keys
            .len()
            .try_into()
            .map_err(|_| AccessError::SolutionDataOutOfBounds)?,
    )?;
    Ok(())
}

pub(crate) fn mut_keys_contains(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    let found = stack.pop_len_words::<_, bool, crate::error::OpError>(|words| {
        Ok(solution.mutable_keys.contains(words))
    })?;
    stack.push(Word::from(found))?;
    Ok(())
}

/// `Access::State` implementation.
pub(crate) fn state(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    let [slot, delta] = stack.pop2()?;
    let slot = state_slot(slots, slot, delta)?;
    stack.extend(slot.clone())?;
    Ok(())
}

/// `Access::StateRange` implementation.
pub(crate) fn state_range(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    let [slot, len, delta] = stack.pop3()?;
    let slice = state_slot_range(slots, slot, len, delta)?;
    for slot in slice {
        stack.extend(slot.clone())?;
    }
    Ok(())
}

/// `Access::StateLen` implementation.
pub(crate) fn state_len(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    stack.pop2_push1(|slot, delta| {
        let slot = state_slot(slots, slot, delta)?;
        let len = Word::try_from(slot.len()).map_err(|_| AccessError::StateSlotOutOfBounds)?;
        Ok(len)
    })
}

/// `Access::StateLenRange` implementation.
pub(crate) fn state_len_range(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    let [slot, len, delta] = stack.pop3()?;
    let slice = state_slot_range(slots, slot, len, delta)?;
    for slot in slice {
        let len = Word::try_from(slot.len()).map_err(|_| AccessError::StateSlotOutOfBounds)?;
        stack.push(len)?;
    }
    Ok(())
}

/// `Access::ThisAddress` implementation.
pub(crate) fn this_address(data: &SolutionData, stack: &mut Stack) -> OpResult<()> {
    let words = word_4_from_u8_32(data.intent_to_solve.intent.0);
    stack.extend(words)?;
    Ok(())
}

/// `Access::ThisSetAddress` implementation.
pub(crate) fn this_set_address(data: &SolutionData, stack: &mut Stack) -> OpResult<()> {
    let words = word_4_from_u8_32(data.intent_to_solve.set.0);
    stack.extend(words)?;
    Ok(())
}

/// `Access::ThisPathway` implementation.
pub(crate) fn this_pathway(index: usize, stack: &mut Stack) -> OpResult<()> {
    let index: Word = index
        .try_into()
        .map_err(|_| AccessError::SolutionDataOutOfBounds)?;
    Ok(stack.push(index)?)
}

pub(crate) fn repeat_counter(stack: &mut Stack, repeat: &Repeat) -> OpResult<()> {
    let counter = repeat.counter()?;
    Ok(stack.push(counter)?)
}

pub(crate) fn transient(stack: &mut Stack, solution: SolutionAccess) -> OpResult<()> {
    let pathway = stack.pop()?;
    let pathway =
        SolutionDataIndex::try_from(pathway).map_err(|_| AccessError::TransientDataOutOfBounds)?;
    let value = stack.pop_len_words::<_, _, StackError>(|key| {
        let value = solution
            .transient_data
            .get(&pathway)
            .ok_or(StackError::IndexOutOfBounds)?
            .get(key)
            .ok_or(StackError::IndexOutOfBounds)?;
        Ok(value.clone())
    })?;
    Ok(stack.extend(value)?)
}

pub(crate) fn transient_len(stack: &mut Stack, solution: SolutionAccess) -> OpResult<()> {
    let pathway = stack.pop()?;
    let pathway =
        SolutionDataIndex::try_from(pathway).map_err(|_| AccessError::TransientDataOutOfBounds)?;
    let length = stack.pop_len_words::<_, _, StackError>(|key| {
        let value = solution
            .transient_data
            .get(&pathway)
            .ok_or(StackError::IndexOutOfBounds)?
            .get(key)
            .ok_or(StackError::IndexOutOfBounds)?;
        Ok(value.len())
    })?;
    let length = Word::try_from(length).map_err(|_| AccessError::TransientDataOutOfBounds)?;
    Ok(stack.push(length)?)
}

pub(crate) fn intent_at(stack: &mut Stack, data: &[SolutionData]) -> OpResult<()> {
    let pathway = stack.pop()?;
    let pathway = usize::try_from(pathway).map_err(|_| AccessError::TransientDataOutOfBounds)?;
    let address = data
        .get(pathway)
        .ok_or(StackError::IndexOutOfBounds)?
        .intent_to_solve
        .clone();
    let set_address = word_4_from_u8_32(address.set.0);
    let intent_address = word_4_from_u8_32(address.intent.0);
    stack.extend(set_address)?;
    stack.extend(intent_address)?;
    Ok(())
}

pub(crate) fn this_transient_len(
    stack: &mut Stack,
    transient_data: Option<&HashMap<Key, Vec<Word>>>,
) -> OpResult<()> {
    let Some(transient_data) = transient_data else {
        return Ok(stack.push(0)?);
    };
    let length = transient_data.len();
    let length = Word::try_from(length).map_err(|_| AccessError::TransientDataOutOfBounds)?;
    stack.push(length)?;
    Ok(())
}

pub(crate) fn this_transient_contains(
    stack: &mut Stack,
    transient_data: Option<&HashMap<Key, Vec<Word>>>,
) -> OpResult<()> {
    let Some(transient_data) = transient_data else {
        stack.pop_len_words::<_, _, StackError>(|_| Ok(()))?;
        return Ok(stack.push(Word::from(false))?);
    };
    let contains =
        stack.pop_len_words::<_, _, StackError>(|key| Ok(transient_data.contains_key(key)))?;
    let contains = Word::from(contains);
    stack.push(contains)?;
    Ok(())
}
/// Resolve the decision variable by traversing any necessary transient data.
///
/// Errors if the solution data or decision var indices are out of bounds
/// (whether provided directly or via a transient decision var) or if a cycle
/// occurs between transient decision variables.
pub(crate) fn resolve_decision_var(
    data: &[SolutionData],
    data_ix: usize,
    var_ix: usize,
) -> Result<Word, AccessError> {
    let solution_data = data
        .get(data_ix)
        .ok_or(AccessError::SolutionDataOutOfBounds)?;
    solution_data
        .decision_variables
        .get(var_ix)
        .copied()
        .ok_or(AccessError::DecisionSlotOutOfBounds)
}

fn state_slot(slots: StateSlots, slot: Word, delta: Word) -> OpResult<&Vec<Word>> {
    let delta = bool_from_word(delta).ok_or(AccessError::InvalidStateSlotDelta(delta))?;
    let slots = state_slots_from_delta(slots, delta);
    let ix = usize::try_from(slot).map_err(|_| AccessError::StateSlotOutOfBounds)?;
    let slot = slots.get(ix).ok_or(AccessError::StateSlotOutOfBounds)?;
    Ok(slot)
}

pub(crate) fn state_slot_range(
    slots: StateSlots,
    slot: Word,
    len: Word,
    delta: Word,
) -> OpResult<&StateSlotSlice> {
    let delta = bool_from_word(delta).ok_or(AccessError::InvalidStateSlotDelta(slot))?;
    let slots = state_slots_from_delta(slots, delta);
    let range = range_from_start_len(slot, len).ok_or(AccessError::StateSlotOutOfBounds)?;
    let subslice = slots
        .get(range)
        .ok_or(AccessError::DecisionSlotOutOfBounds)?;
    Ok(subslice)
}

fn range_from_start_len(start: Word, len: Word) -> Option<std::ops::Range<usize>> {
    let start = usize::try_from(start).ok()?;
    let len = usize::try_from(len).ok()?;
    let end = start.checked_add(len)?;
    Some(start..end)
}

fn state_slots_from_delta(slots: StateSlots, delta: bool) -> &StateSlotSlice {
    if delta {
        slots.post
    } else {
        slots.pre
    }
}
