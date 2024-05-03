use crate::{
    state_read_vm::{Gas, StateRead},
    types::{intent::Intent, solution::Solution, ContentAddress, IntentAddress, Key, Word},
};
pub use essential_constraint_vm as constraint_vm;
pub use essential_state_read_vm as state_read_vm;
pub use essential_types as types;
pub use solution::Utility;
use std::sync::Arc;

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
    intents: impl Fn(&IntentAddress) -> Option<Arc<Intent>>,
) -> anyhow::Result<(S, Utility, Gas)>
where
    S: Clone + Send + Sync + StateRead + StateTransactionWrite + 'static,
    S::Future: Send,
{
    let post_state = apply_mutation(pre_state, &solution)?;
    solution::check_intents(pre_state, &post_state, solution, intents)
        .await
        .map(|(utility, gas)| (post_state, utility, gas))
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
