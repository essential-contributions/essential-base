//! Access operation implementations.

use crate::{
    cached::LazyCache,
    error::{AccessError, MissingAccessArgError},
    repeat::Repeat,
    sets::encode_set,
    OpResult, Stack,
};
use essential_constraint_asm::Word;
use essential_types::{
    convert::{bytes_from_word, u8_32_from_word_4, word_4_from_u8_32},
    solution::{Solution, SolutionData, SolutionDataIndex},
    Key, Value,
};
use std::collections::HashSet;

#[cfg(test)]
mod dec_vars;
#[cfg(test)]
mod predicate_exists;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;

/// All necessary solution data access required to check an individual predicate.
#[derive(Clone, Copy, Debug)]
pub struct Access<'a> {
    /// The input data for each predicate being solved within the solution.
    ///
    /// We require *all* predicate solution data in order to handle checking
    /// predicate exists.
    pub data: &'a [SolutionData],
    /// Checking is performed for one predicate at a time. This index refers to
    /// the checked predicate's associated solution data within `data`.
    pub index: usize,
    /// The keys being proposed for mutation for the predicate.
    pub mutable_keys: &'a HashSet<&'a [Word]>,
}

impl<'a> Access<'a> {
    /// A shorthand for constructing a `SolutionAccess` instance for checking
    /// the predicate at the given index within the given solution.
    ///
    /// This constructor assumes that the given mutable keys contract is correct
    /// for this solution. It is not checked by this function for performance.
    pub fn new(
        solution: &'a Solution,
        predicate_index: SolutionDataIndex,
        mutable_keys: &'a HashSet<&[Word]>,
    ) -> Self {
        Self {
            data: &solution.data,
            index: predicate_index.into(),
            mutable_keys,
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
pub(crate) fn push_mut_keys(access: Access, stack: &mut Stack) -> OpResult<()> {
    encode_set(access.mutable_keys.iter().map(|k| k.iter().copied()), stack)
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

pub(crate) fn repeat_counter(stack: &mut Stack, repeat: &Repeat) -> OpResult<()> {
    let counter = repeat.counter()?;
    Ok(stack.push(counter)?)
}

/// Implementation of the `Access::NumSlots` operation.
pub(crate) fn decision_var_slots(stack: &mut Stack, decision_variables: &[Value]) -> OpResult<()> {
    let num_slots = Word::try_from(decision_variables.len())
        .map_err(|_| AccessError::SlotsLengthTooLarge(decision_variables.len()))?;
    stack.push(num_slots)?;
    Ok(())
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

pub(crate) fn predicate_exists(
    stack: &mut Stack,
    data: &[SolutionData],
    cache: &LazyCache,
) -> OpResult<()> {
    let hash = u8_32_from_word_4(stack.pop4()?);
    let found = cache.get_dec_var_hashes(data).contains(&hash);
    stack.push(found as Word)?;
    Ok(())
}

pub(crate) fn init_predicate_exists(
    data: &[SolutionData],
) -> impl Iterator<Item = essential_types::Hash> + '_ {
    data.iter().map(|d| {
        let data = d
            .decision_variables
            .iter()
            .flat_map(|slot| {
                Some(slot.len() as Word)
                    .into_iter()
                    .chain(slot.iter().cloned())
            })
            .chain(word_4_from_u8_32(d.predicate_to_solve.contract.0))
            .chain(word_4_from_u8_32(d.predicate_to_solve.predicate.0))
            .flat_map(bytes_from_word)
            .collect::<Vec<_>>();
        sha256(&data)
    })
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result: [u8; 32] = hasher.finalize().into();
    result
}

fn range_from_start_len(start: Word, len: Word) -> Option<std::ops::Range<usize>> {
    let start = usize::try_from(start).ok()?;
    let len = usize::try_from(len).ok()?;
    let end = start.checked_add(len)?;
    Some(start..end)
}
