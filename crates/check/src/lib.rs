//! Core logic for validating [`Intent`]s, [`Solution`]s and
//! [`SolutionData`][crate::types::solution::SolutionData] against associated intents.
//!
//! ## Intent Validation
//!
//! - [`intent::check_signed_set`] validates a signed set of intents. Also exposes:
//! - [`intent::check_set`] validates a set of intents.
//! - [`intent::check`] validate an individual intent.
//! - [`intent::check_slots`] validate an intent's slots.
//! - [`intent::check_directive`] validate an intent's directive.
//! - [`intent::check_state_reads`] validate an intent's state read bytecode.
//! - [`intent::check_constraints`] validate an intent's constraint bytecode.
//!
//! ## Solution Validation
//!
//! - [`solution::check_signed`] validates a signed solution.
//! - [`solution::check`] validates an unsigned solution.
//! - [`solution::check_data`] validates a solution's data slice.
//! - [`solution::check_state_mutations`] validates a solution's state mutation slice.
//! - [`solution::check_partial_solutions`] validates a solution's signed partial solutions.
//!
//! ## Solution + Intent Validation
//!
//! - [`solution::check_intents`] validates a solution's data against their associated intents.
//! - [`solution::check_intent`] validates a single solution data against an associated intent.
//! - [`solution::check_intent_constraints`] the intent constraint checking part of solution
//!   data validation.

#![deny(missing_docs)]
#![deny(unsafe_code)]

use crate::{
    state_read_vm::{Gas, StateRead},
    types::{intent::Intent, solution::Solution, ContentAddress, IntentAddress, Key, Word},
};
#[doc(inline)]
pub use essential_constraint_vm as constraint_vm;
#[doc(inline)]
pub use essential_sign as sign;
#[doc(inline)]
pub use essential_state_read_vm as state_read_vm;
#[doc(inline)]
pub use essential_types as types;
#[doc(inline)]
pub use solution::Utility;
use std::sync::Arc;

pub mod intent;
pub mod solution;

// TODO: Remove this `StateTransactionWrite` and `apply_mutation` stuff. It's
// just a temporary demo on how the state mutations can be applied separately.

// be separated out from the solution checking.
/// State transaction types that may be written to.
pub trait StateTransactionWrite {
    /// Update the entry at the given key associated with the given intent set.
    ///
    /// Note that this is not asynchronous, should *never* fail, and must return immediately.
    fn write_word(&mut self, set_addr: &ContentAddress, key: &Key, value: Option<Word>);
}

/// Shorthand for `apply_mutation` then `check_solution` with the resulting `post_state`.
///
/// Returns the resulting `post_state` alongside the utility and gas.
pub async fn apply_mutation_and_check_solution<S>(
    pre_state: &S,
    solution: Arc<Solution>,
    intents: impl Fn(&IntentAddress) -> Arc<Intent>,
) -> anyhow::Result<(S, Utility, Gas)>
where
    S: Clone + Send + Sync + StateRead + StateTransactionWrite + 'static,
    S::Future: Send,
    S::Error: Send,
{
    let post_state = apply_mutation(pre_state, &solution)?;
    solution::check_intents(pre_state, &post_state, solution, intents)
        .await
        .map(|(utility, gas)| (post_state, utility, gas))
        .map_err(|e| anyhow::Error::msg(e.to_string()))
}

/// Clones the given `pre_state`, applies the mutation and returns the resulting `post_state`.
pub fn apply_mutation<S>(pre_state: &S, solution: &Solution) -> anyhow::Result<S>
where
    S: Clone + StateTransactionWrite,
{
    let mut post_state = pre_state.clone();
    for state_mutation in &solution.state_mutations {
        let set = &solution
            .data
            .get(state_mutation.pathway as usize)
            .ok_or(anyhow::anyhow!("Intent in solution data not found"))?
            .intent_to_solve
            .set;
        for mutation in state_mutation.mutations.iter() {
            post_state.write_word(set, &mutation.key, mutation.value);
        }
    }
    Ok(post_state)
}
