use crate::access::init_predicate_exists;
use essential_types::{solution::Solution, Hash};
use std::{collections::HashSet, sync::OnceLock};

#[derive(Default, Debug, PartialEq)]
/// Lazily cache expensive to compute values.
pub struct LazyCache {
    /// Predicate data and addresses set of hashes.
    /// See [`PredicateExists`][essential_asm] for more details.
    pub pred_data_hashes: OnceLock<HashSet<Hash>>,
}

impl LazyCache {
    /// Create a new empty `LazyCache`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the predicate data hashes.
    ///
    /// The first time this is called, it will compute the hashes.
    pub fn get_pred_data_hashes(&self, solutions: &[Solution]) -> &HashSet<Hash> {
        self.pred_data_hashes
            .get_or_init(|| init_predicate_exists(solutions).collect())
    }
}
