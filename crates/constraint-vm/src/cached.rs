use std::{collections::HashSet, sync::OnceLock};

use essential_types::{solution::SolutionData, Hash};

use crate::access::init_predicate_exists;

#[derive(Default, Debug, PartialEq)]
/// Lazily cache expensive to compute values.
pub struct LazyCache {
    /// Decision variables and addresses set of hashes.
    pub dec_var_hashes: OnceLock<HashSet<Hash>>,
}

impl LazyCache {
    /// Create a new empty `LazyCache`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the decision variable hashes.
    ///
    /// The first time this is called, it will compute the hashes.
    pub fn get_dec_var_hashes(&self, data: &[SolutionData]) -> &HashSet<Hash> {
        self.dec_var_hashes
            .get_or_init(|| init_predicate_exists(data).collect())
    }
}
