//! # Solutions
//! Data types that are used to create solutions to predicates.

use serde::{Deserialize, Serialize};

use crate::{Key, PredicateAddress, Value};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

/// An index into a [`SolutionSet`]'s `solutions` slice.
///
/// Note that this type is purely provided as a utility. Implementations should not depend on the
/// order of `Solution`s within a `SolutionSet` as it must be possible for `SolutionSet`s to be
/// merged.
pub type SolutionIndex = u16;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A set of [`Solution`]s.
///
/// A `SolutionSet`'s `ContentAddress` is the same regardless of the ordering of its solutions.
///
/// `SolutionSet`s may be safely merged with one another in the case that there are no [`Key`]
/// conflicts in the proposed [`state_mutations`][Solution::state_mutations] and/or post-state
/// reads within the [`predicate_to_solve`][Solution::predicate_to_solve].
pub struct SolutionSet {
    /// The input data for each predicate.
    // Support deserializing the old `data` name.
    #[serde(alias = "data")]
    pub solutions: Vec<Solution>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A solution for a single contract predicate.
pub struct Solution {
    /// The predicate that the solution attempts to solve.
    pub predicate_to_solve: PredicateAddress,
    /// The input data required by the predicate.
    // Support deserializing the old `decision_variables` name.
    #[serde(alias = "decision_variables")]
    pub predicate_data: Vec<Value>,
    /// The state mutations proposed by the solution.
    pub state_mutations: Vec<Mutation>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// A mutation to a single [`Key`] in state.
pub struct Mutation {
    /// Key to data.
    pub key: Key,
    /// The new value.
    ///
    /// Empty value means the value is being deleted.
    pub value: Value,
}

impl SolutionSet {
    /// Get the sum of all state mutations within the set of solutions.
    pub fn state_mutations_len(&self) -> usize {
        self.solutions.iter().map(|d| d.state_mutations.len()).sum()
    }
}
