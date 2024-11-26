//! Access operation implementations.

use crate::{
    cached::LazyCache,
    error::{AccessError, MissingAccessArgError, OpSyncResult},
    repeat::Repeat,
    sets::encode_set,
    types::{
        convert::{bytes_from_word, u8_32_from_word_4, word_4_from_u8_32},
        solution::{Solution, SolutionIndex, SolutionSet},
        Key, Value, Word,
    },
    Stack,
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
    /// The set of input data for each predicate being solved within the solution set.
    ///
    /// We require *all* solutions in order to handle checking predicate exists.
    pub solutions: &'a [Solution],
    /// Checking is performed for one solution at a time. This index refers to
    /// the checked predicate's associated solution within the `SolutionSet` slice.
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
        set: &'a SolutionSet,
        solution_index: SolutionIndex,
        mutable_keys: &'a HashSet<&[Word]>,
    ) -> Self {
        Self {
            solutions: &set.solutions,
            index: solution_index.into(),
            mutable_keys,
        }
    }

    /// The solution associated with the predicate currently being checked.
    ///
    /// **Panics** in the case that `self.index` is out of range of the `self.solutions` slice.
    pub fn this_solution(&self) -> &Solution {
        self.solutions
            .get(self.index)
            .expect("solution index out of range of solutions slice")
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
pub fn mut_keys(set: &SolutionSet, solution_index: SolutionIndex) -> impl Iterator<Item = &Key> {
    set.solutions[solution_index as usize]
        .state_mutations
        .iter()
        .map(|m| &m.key)
}

/// Get the mutable keys as slices
pub fn mut_keys_slices(
    set: &SolutionSet,
    solution_index: SolutionIndex,
) -> impl Iterator<Item = &[Word]> {
    set.solutions[solution_index as usize]
        .state_mutations
        .iter()
        .map(|m| m.key.as_ref())
}

/// Get the contract of mutable keys for this predicate.
pub fn mut_keys_set(solution_set: &SolutionSet, solution_index: SolutionIndex) -> HashSet<&[Word]> {
    mut_keys_slices(solution_set, solution_index).collect()
}

/// `Access::PredicateData` implementation.
pub(crate) fn predicate_data(this_predicate_data: &[Value], stack: &mut Stack) -> OpSyncResult<()> {
    let len = stack
        .pop()
        .map_err(|_| AccessError::MissingArg(MissingAccessArgError::PredDataLen))?;
    let value_ix = stack
        .pop()
        .map_err(|_| AccessError::MissingArg(MissingAccessArgError::PredDataValueIx))?;
    let slot_ix = stack
        .pop()
        .map_err(|_| AccessError::MissingArg(MissingAccessArgError::PredDataSlotIx))?;
    let slot_ix = usize::try_from(slot_ix)
        .map_err(|_| AccessError::PredicateDataSlotIxOutOfBounds(slot_ix))?;
    let range = range_from_start_len(value_ix, len).ok_or(AccessError::InvalidAccessRange)?;
    let words = resolve_predicate_data_range(this_predicate_data, slot_ix, range)?;
    stack.extend(words.iter().copied())?;
    Ok(())
}

/// `Access::PredicateDataLen` implementation.
pub(crate) fn predicate_data_len(
    this_predicate_data: &[Value],
    stack: &mut Stack,
) -> Result<(), AccessError> {
    let slot_ix = stack
        .pop()
        .map_err(|_| MissingAccessArgError::PredDataSlotIx)?;
    let slot_ix = usize::try_from(slot_ix)
        .map_err(|_| AccessError::PredicateDataSlotIxOutOfBounds(slot_ix))?;
    let len = resolve_predicate_data_len(this_predicate_data, slot_ix)?;
    let w = Word::try_from(len).map_err(|_| AccessError::PredicateDataValueTooLarge(len))?;
    stack
        .push(w)
        .expect("Can't fail because 1 is popped and 1 is pushed");
    Ok(())
}

/// `Access::MutKeys` implementation.
pub(crate) fn push_mut_keys(access: Access, stack: &mut Stack) -> OpSyncResult<()> {
    encode_set(access.mutable_keys.iter().map(|k| k.iter().copied()), stack)
}

/// `Access::ThisAddress` implementation.
pub(crate) fn this_address(solution: &Solution, stack: &mut Stack) -> OpSyncResult<()> {
    let words = word_4_from_u8_32(solution.predicate_to_solve.predicate.0);
    stack.extend(words)?;
    Ok(())
}

/// `Access::ThisContractAddress` implementation.
pub(crate) fn this_contract_address(solution: &Solution, stack: &mut Stack) -> OpSyncResult<()> {
    let words = word_4_from_u8_32(solution.predicate_to_solve.contract.0);
    stack.extend(words)?;
    Ok(())
}

pub(crate) fn repeat_counter(stack: &mut Stack, repeat: &Repeat) -> OpSyncResult<()> {
    let counter = repeat.counter()?;
    Ok(stack.push(counter)?)
}

/// Implementation of the `Access::NumSlots` operation.
pub(crate) fn predicate_data_slots(
    stack: &mut Stack,
    predicate_data: &[Value],
) -> OpSyncResult<()> {
    let num_slots = Word::try_from(predicate_data.len())
        .map_err(|_| AccessError::SlotsLengthTooLarge(predicate_data.len()))?;
    stack.push(num_slots)?;
    Ok(())
}

/// Resolve a range of words at a predicate data slot.
///
/// Errors if the solution or predicate data indices are out of bounds.
pub(crate) fn resolve_predicate_data_range(
    predicate_data: &[Value],
    slot_ix: usize,
    value_range_ix: core::ops::Range<usize>,
) -> Result<&[Word], AccessError> {
    predicate_data
        .get(slot_ix)
        .ok_or(AccessError::PredicateDataSlotIxOutOfBounds(slot_ix as Word))?
        .get(value_range_ix.clone())
        .ok_or(AccessError::PredicateDataSlotRangeOutOfBounds(
            value_range_ix.start as Word,
            value_range_ix.end as Word,
        ))
}

/// Resolve the length of predicate data slot.
///
/// Errors if the solution or decision var indices are out of bounds.
pub(crate) fn resolve_predicate_data_len(
    predicate_data: &[Value],
    slot_ix: usize,
) -> Result<usize, AccessError> {
    predicate_data
        .get(slot_ix)
        .map(|slot| slot.len())
        .ok_or(AccessError::PredicateDataSlotIxOutOfBounds(slot_ix as Word))
}

pub(crate) fn predicate_exists(
    stack: &mut Stack,
    solutions: &[Solution],
    cache: &LazyCache,
) -> OpSyncResult<()> {
    let hash = u8_32_from_word_4(stack.pop4()?);
    let found = cache.get_pred_data_hashes(solutions).contains(&hash);
    stack.push(found as Word)?;
    Ok(())
}

pub(crate) fn init_predicate_exists(
    solutions: &[Solution],
) -> impl Iterator<Item = essential_types::Hash> + '_ {
    solutions.iter().map(|d| {
        let data = d
            .predicate_data
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
