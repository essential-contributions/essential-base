//! Access operation implementations.

use crate::{
    error::{AccessError, LenWordsError, MissingAccessArgError, OpError, StackError},
    repeat::Repeat,
    sets::encode_set,
    types::convert::bool_from_word,
    OpResult, Stack,
};
use essential_constraint_asm::Word;
use essential_types::{
    convert::word_4_from_u8_32,
    solution::{Mutation, Solution, SolutionData, SolutionDataIndex},
    Key, Value,
};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod dec_vars;
#[cfg(test)]
mod pub_vars;
#[cfg(test)]
mod state;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;

/// Transient data map.
pub type TransientData = HashMap<SolutionDataIndex, HashMap<Key, Vec<Word>>>;

/// All necessary solution data and state access required to check an individual predicate.
#[derive(Clone, Copy, Debug)]
pub struct Access<'a> {
    /// All necessary solution data access required to check an individual predicate.
    pub solution: SolutionAccess<'a>,
    /// The pre and post mutation state slot values for the predicate being solved.
    pub state_slots: StateSlots<'a>,
}

/// All necessary solution data access required to check an individual predicate.
#[derive(Clone, Copy, Debug)]
pub struct SolutionAccess<'a> {
    /// The input data for each predicate being solved within the solution.
    ///
    /// We require *all* predicate solution data in order to handle transient
    /// decision variable access.
    pub data: &'a [SolutionData],
    /// Checking is performed for one predicate at a time. This index refers to
    /// the checked predicate's associated solution data within `data`.
    pub index: usize,
    /// The keys being proposed for mutation for the predicate.
    pub mutable_keys: &'a HashSet<&'a [Word]>,
    /// The transient data that points to the data in another solution data index.
    pub transient_data: &'a TransientData,
}

/// The pre and post mutation state slot values for the predicate being solved.
#[derive(Clone, Copy, Debug)]
pub struct StateSlots<'a> {
    /// Predicate state slot values before the solution's mutations are applied.
    pub pre: &'a StateSlotSlice,
    /// Predicate state slot values after the solution's mutations are applied.
    pub post: &'a StateSlotSlice,
}

/// The state slots declared within the predicate.
pub type StateSlotSlice = [Vec<Word>];

impl<'a> SolutionAccess<'a> {
    /// A shorthand for constructing a `SolutionAccess` instance for checking
    /// the predicate at the given index within the given solution.
    ///
    /// This constructor assumes that the given mutable keys contract is correct
    /// for this solution. It is not checked by this function for performance.
    pub fn new(
        solution: &'a Solution,
        predicate_index: SolutionDataIndex,
        mutable_keys: &'a HashSet<&[Word]>,
        transient_data: &'a TransientData,
    ) -> Self {
        Self {
            data: &solution.data,
            index: predicate_index.into(),
            mutable_keys,
            transient_data,
        }
    }

    /// The solution data associated with the predicate currently being checked.
    ///
    /// **Panics** in the case that `self.index` is out of range of the `self.data` slice.
    pub fn this_data(&self) -> &SolutionData {
        self.data
            .get(self.index)
            .expect("predicate index out of range of solution data")
    }

    /// The transient data associated with the predicate currently being checked.
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
/// the predicate at the given index.
///
/// Specifically, assists in calculating the `mut_keys_len` for
/// `SolutionAccess`, as this is equal to the `.count()` of the returned iterator.
///
/// **Note:** In the case that the given solution is invalid and contains multiple
/// mutations to the same key, the same key will be yielded multiple times.
pub fn mut_keys(
    solution: &Solution,
    predicate_index: SolutionDataIndex,
) -> impl Iterator<Item = &Key> {
    solution.data[predicate_index as usize]
        .state_mutations
        .iter()
        .map(|m| &m.key)
}

/// Get the mutable keys as slices
pub fn mut_keys_slices(
    solution: &Solution,
    predicate_index: SolutionDataIndex,
) -> impl Iterator<Item = &[Word]> {
    solution.data[predicate_index as usize]
        .state_mutations
        .iter()
        .map(|m| m.key.as_ref())
}

/// Get the contract of mutable keys for this predicate.
pub fn mut_keys_set(solution: &Solution, predicate_index: SolutionDataIndex) -> HashSet<&[Word]> {
    mut_keys_slices(solution, predicate_index).collect()
}

/// Create a transient data map from the solution.
pub fn transient_data(solution: &Solution) -> TransientData {
    let mut transient_data = HashMap::new();
    for (ix, data) in solution.data.iter().enumerate() {
        if !data.transient_data.is_empty() {
            transient_data.insert(
                ix as SolutionDataIndex,
                data.transient_data
                    .iter()
                    .cloned()
                    .map(|Mutation { key, value }| (key, value))
                    .collect(),
            );
        }
    }
    transient_data
}

/// `Access::DecisionVar` implementation.
pub(crate) fn decision_var(this_decision_vars: &[Value], stack: &mut Stack) -> OpResult<()> {
    let len = stack.pop().map_err(|_| MissingAccessArgError::DecVarLen)?;
    let value_ix = stack
        .pop()
        .map_err(|_| MissingAccessArgError::DecVarValueIx)?;
    let slot_ix = stack
        .pop()
        .map_err(|_| MissingAccessArgError::DecVarSlotIx)?;
    let slot_ix =
        usize::try_from(slot_ix).map_err(|_| AccessError::DecisionSlotIxOutOfBounds(slot_ix))?;
    let range = range_from_start_len(value_ix, len).ok_or(AccessError::InvalidAccessRange)?;
    let words = resolve_decision_var_range(this_decision_vars, slot_ix, range)?;
    stack.extend(words.iter().copied())?;
    Ok(())
}

/// `Access::DecisionVarLen` implementation.
pub(crate) fn decision_var_len(this_decision_vars: &[Value], stack: &mut Stack) -> OpResult<()> {
    let slot_ix = stack
        .pop()
        .map_err(|_| MissingAccessArgError::DecVarSlotIx)?;
    let slot_ix =
        usize::try_from(slot_ix).map_err(|_| AccessError::DecisionSlotIxOutOfBounds(slot_ix))?;
    let len = resolve_decision_var_len(this_decision_vars, slot_ix)?;
    let w = Word::try_from(len).map_err(|_| AccessError::DecisionLengthTooLarge(len))?;
    stack
        .push(w)
        .expect("Can't fail because 1 is popped and 1 is pushed");
    Ok(())
}

/// `Access::MutKeys` implementation.
pub(crate) fn push_mut_keys(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    encode_set(
        solution.mutable_keys.iter().map(|k| k.iter().copied()),
        stack,
    )
}

/// `Access::PubVarKeys` implementation.
pub(crate) fn push_pub_var_keys(pub_vars: &TransientData, stack: &mut Stack) -> OpResult<()> {
    let pathway_ix = stack
        .pop()
        .map_err(|_| MissingAccessArgError::PushPubVarKeysPathwayIx)?;
    let pathway_ix = SolutionDataIndex::try_from(pathway_ix)
        .map_err(|_| AccessError::PathwayOutOfBounds(pathway_ix))?;
    let pub_vars = pub_vars
        .get(&pathway_ix)
        .ok_or(AccessError::PathwayOutOfBounds(pathway_ix as Word))?;

    encode_set(pub_vars.keys().map(|k| k.iter().copied()), stack)?;

    Ok(())
}

/// `Access::State` implementation.
pub(crate) fn state(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    let delta = stack.pop().map_err(|_| MissingAccessArgError::StateDelta)?;
    let len = stack.pop().map_err(|_| MissingAccessArgError::StateLen)?;
    let value_ix = stack
        .pop()
        .map_err(|_| MissingAccessArgError::StateValueIx)?;
    let slot_ix = stack
        .pop()
        .map_err(|_| MissingAccessArgError::StateSlotIx)?;
    let values = state_slot_value_range(slots, slot_ix, value_ix, len, delta)?;
    stack.extend(values.iter().copied())?;
    Ok(())
}

/// `Access::StateLen` implementation.
pub(crate) fn state_len(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    let delta = stack.pop().map_err(|_| MissingAccessArgError::StateDelta)?;
    let slot_ix = stack
        .pop()
        .map_err(|_| MissingAccessArgError::StateSlotIx)?;
    let slot = state_slot(slots, slot_ix, delta)?;
    let len =
        Word::try_from(slot.len()).map_err(|_| AccessError::StateValueTooLarge(slot.len()))?;
    stack
        .push(len)
        .expect("Can't fail because 2 are popped and 1 is pushed");
    Ok(())
}

/// `Access::ThisAddress` implementation.
pub(crate) fn this_address(data: &SolutionData, stack: &mut Stack) -> OpResult<()> {
    let words = word_4_from_u8_32(data.predicate_to_solve.predicate.0);
    stack.extend(words)?;
    Ok(())
}

/// `Access::ThisContractAddress` implementation.
pub(crate) fn this_contract_address(data: &SolutionData, stack: &mut Stack) -> OpResult<()> {
    let words = word_4_from_u8_32(data.predicate_to_solve.contract.0);
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

/// `Access::PubVar` implementation.
pub(crate) fn pub_var(stack: &mut Stack, pub_vars: &TransientData) -> OpResult<()> {
    // Pop the value_len, value_ix and create a range.
    let value_len = stack
        .pop()
        .map_err(|_| MissingAccessArgError::PubVarValueLen)?;
    let value_ix = stack
        .pop()
        .map_err(|_| MissingAccessArgError::PubVarValueIx)?;
    let range = range_from_start_len(value_ix, value_len).ok_or(AccessError::InvalidAccessRange)?;

    let key_length = stack
        .pop()
        .map_err(|_| MissingAccessArgError::PubVarKeyLen)?;
    let length = usize::try_from(key_length)
        .map_err(|_| AccessError::KeyLengthOutOfBounds(key_length))?
        .checked_add(1)
        .ok_or(AccessError::KeyLengthOutOfBounds(key_length))?;

    // Pop the key and access the value.
    let value = stack
        .pop_words::<_, _, OpError>(length, |slice| {
            let (pathway_ix, key) = slice
                .split_first()
                .expect("Can't fail because must have at least 1 word");
            let pathway_ix = SolutionDataIndex::try_from(*pathway_ix)
                .map_err(|_| AccessError::PathwayOutOfBounds(*pathway_ix))?;
            let value = pub_vars
                .get(&pathway_ix)
                .ok_or(AccessError::PathwayOutOfBounds(pathway_ix as Word))?
                .get(key)
                .ok_or(AccessError::PubVarKeyOutOfBounds)?
                .get(range)
                .ok_or(AccessError::PubVarDataOutOfBounds)?;
            Ok(value.to_vec())
        })
        .map_err(map_key_len_err)?;

    Ok(stack.extend(value)?)
}

pub(crate) fn pub_var_len(stack: &mut Stack, pub_vars: &TransientData) -> OpResult<()> {
    let key_length = stack
        .pop()
        .map_err(|_| MissingAccessArgError::PubVarKeyLen)?;
    let length = usize::try_from(key_length)
        .map_err(|_| AccessError::KeyLengthOutOfBounds(key_length))?
        .checked_add(1)
        .ok_or(AccessError::KeyLengthOutOfBounds(key_length))?;
    // Pop the key and get the length of the value.
    let length = stack
        .pop_words::<_, _, OpError>(length, |slice| {
            let (pathway_ix, key) = slice
                .split_first()
                .expect("Can't fail because must have at least 1 word");
            let pathway_ix = SolutionDataIndex::try_from(*pathway_ix)
                .map_err(|_| AccessError::PathwayOutOfBounds(*pathway_ix))?;
            let value = pub_vars
                .get(&pathway_ix)
                .ok_or(AccessError::PathwayOutOfBounds(pathway_ix as Word))?
                .get(key)
                .ok_or(AccessError::PubVarKeyOutOfBounds)?;
            Ok(value.len())
        })
        .map_err(map_key_len_err)?;

    let length = Word::try_from(length).map_err(|_| AccessError::PubVarDataOutOfBounds)?;

    stack
        .push(length)
        .expect("Can't fail because 3 are popped and 1 is pushed");

    Ok(())
}

pub(crate) fn predicate_at(stack: &mut Stack, data: &[SolutionData]) -> OpResult<()> {
    let pathway = stack.pop()?;
    let pathway = usize::try_from(pathway).map_err(|_| AccessError::PathwayOutOfBounds(pathway))?;
    let address = data
        .get(pathway)
        .ok_or(StackError::IndexOutOfBounds)?
        .predicate_to_solve
        .clone();
    let contract_address = word_4_from_u8_32(address.contract.0);
    let predicate_address = word_4_from_u8_32(address.predicate.0);
    stack.extend(contract_address)?;
    stack.extend(predicate_address)?;
    Ok(())
}

fn map_key_len_err(e: OpError) -> OpError {
    match e {
        OpError::Stack(StackError::LenWords(e)) => match e {
            LenWordsError::OutOfBounds(_) => MissingAccessArgError::PubVarKey.into(),
            LenWordsError::MissingLength => MissingAccessArgError::PubVarKeyLen.into(),
            LenWordsError::InvalidLength(l) => AccessError::KeyLengthOutOfBounds(l).into(),
            e => StackError::LenWords(e).into(),
        },
        e => e,
    }
}

/// Resolve a range of words at a decision variable slot.
///
/// Errors if the solution data or decision var indices are out of bounds.
pub(crate) fn resolve_decision_var_range(
    decision_variables: &[Value],
    slot_ix: usize,
    value_range_ix: core::ops::Range<usize>,
) -> Result<&[Word], AccessError> {
    decision_variables
        .get(slot_ix)
        .ok_or(AccessError::DecisionSlotIxOutOfBounds(slot_ix as Word))?
        .get(value_range_ix.clone())
        .ok_or(AccessError::DecisionValueRangeOutOfBounds(
            value_range_ix.start as Word,
            value_range_ix.end as Word,
        ))
}

/// Resolve the length of decision variable slot.
///
/// Errors if the solution data or decision var indices are out of bounds.
pub(crate) fn resolve_decision_var_len(
    decision_variables: &[Value],
    slot_ix: usize,
) -> Result<usize, AccessError> {
    decision_variables
        .get(slot_ix)
        .map(|slot| slot.len())
        .ok_or(AccessError::DecisionSlotIxOutOfBounds(slot_ix as Word))
}

fn state_slot(slots: StateSlots, slot_ix: Word, delta: Word) -> OpResult<&Vec<Word>> {
    let delta = bool_from_word(delta).ok_or(AccessError::InvalidStateSlotDelta(delta))?;
    let slots = state_slots_from_delta(slots, delta);
    let ix = usize::try_from(slot_ix).map_err(|_| AccessError::StateSlotIxOutOfBounds(slot_ix))?;
    let slot = slots
        .get(ix)
        .ok_or(AccessError::StateSlotIxOutOfBounds(slot_ix))?;
    Ok(slot)
}

fn state_slot_value_range(
    slots: StateSlots,
    slot_ix: Word,
    value_ix: Word,
    len: Word,
    delta: Word,
) -> OpResult<&[Word]> {
    let delta = bool_from_word(delta).ok_or(AccessError::InvalidStateSlotDelta(delta))?;
    let slots = state_slots_from_delta(slots, delta);
    let slot_ix =
        usize::try_from(slot_ix).map_err(|_| AccessError::StateSlotIxOutOfBounds(slot_ix))?;
    let range = range_from_start_len(value_ix, len).ok_or(AccessError::InvalidAccessRange)?;
    let values = slots
        .get(slot_ix)
        .ok_or(AccessError::StateSlotIxOutOfBounds(slot_ix as Word))?
        .get(range.clone())
        .ok_or(AccessError::StateValueRangeOutOfBounds(
            range.start as Word,
            range.end as Word,
        ))?;
    Ok(values)
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
