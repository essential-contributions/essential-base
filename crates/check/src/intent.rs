//! Items related to the validation of [`Intent`]s.

use crate::{
    sign::verify,
    types::{
        intent::{Directive, Intent},
        slots::{state_len, Slots},
        ConstraintBytecode, Signed, StateReadBytecode,
    },
};
use anyhow::ensure;

/// Maximum number of intents that of an intent set.
pub const MAX_INTENTS: usize = 100;
/// Maximum number of state read programs of an intent.
pub const MAX_STATE_READS: usize = 100;
/// Maximum size of state read programs of an intent in bytes.
pub const MAX_STATE_READ_SIZE_IN_BYTES: usize = 10_000;
/// Maximum number of constraint check programs of an intent.
pub const MAX_CONSTRAINTS: usize = 100;
/// Maximum size of constraint check programs of an intent in bytes.
pub const MAX_CONSTRAINT_SIZE_IN_BYTES: usize = 10_000;
/// Maximum number of decision variables of the slots of an intent.
pub const MAX_DECISION_VARIABLES: u32 = 100;
/// Maximum number of state slots of an intent.
pub const MAX_NUM_STATE_SLOTS: usize = 1000;
/// Maximum length of state slots of an intent.
pub const MAX_STATE_LEN: u32 = 1000;
/// Maximum size of directive of an intent.
pub const MAX_DIRECTIVE_SIZE: usize = 1000;

/// Validate a signed set of intents.
///
/// Verifies the signature and then validates the intent set.
pub fn check_signed_set(intents: &Signed<Vec<Intent>>) -> anyhow::Result<()> {
    ensure!(verify(intents), "Failed to verify intent set signature");
    check_set(&intents.data)?;
    Ok(())
}

/// Validate a set of intents
///
/// Checks the size of the set and then validates each intent.
pub fn check_set(intents: &[Intent]) -> anyhow::Result<()> {
    ensure!(intents.len() <= MAX_INTENTS, "Too many intents");
    for intent in intents {
        check(intent)?;
    }
    Ok(())
}

/// Validate a single intent.
///
/// Validates the slots, directive, state reads, and constraints.
pub fn check(intent: &Intent) -> anyhow::Result<()> {
    check_slots(&intent.slots)?;
    check_directive(&intent.directive)?;
    check_state_reads(&intent.state_read)?;
    check_constraints(&intent.constraints)?;
    Ok(())
}

/// Validate an intent's slots.
pub fn check_slots(slots: &Slots) -> anyhow::Result<()> {
    ensure!(
        slots.decision_variables <= MAX_DECISION_VARIABLES,
        "Too many decision variables"
    );
    ensure!(
        slots.state.len() <= MAX_NUM_STATE_SLOTS,
        "Too many state slots"
    );
    let len = state_len(&slots.state);
    ensure!(len.is_some(), "Invalid slots state length");
    ensure!(
        len.unwrap() <= MAX_STATE_LEN,
        "Slots state length too large"
    );
    Ok(())
}

/// Validate an intent's directive.
pub fn check_directive(directive: &Directive) -> anyhow::Result<()> {
    if let Directive::Maximize(program) | Directive::Minimize(program) = directive {
        ensure!(program.len() <= MAX_DIRECTIVE_SIZE, "Directive too large");
    }
    Ok(())
}

/// Validate an intent's state read bytecode.
pub fn check_state_reads(state_reads: &[StateReadBytecode]) -> anyhow::Result<()> {
    ensure!(state_reads.len() <= MAX_STATE_READS, "Too many state reads");
    ensure!(
        state_reads
            .iter()
            .all(|sr| sr.len() <= MAX_STATE_READ_SIZE_IN_BYTES),
        "State read too large"
    );
    Ok(())
}

/// Validate an intent's constraint bytecode.
pub fn check_constraints(constraints: &[ConstraintBytecode]) -> anyhow::Result<()> {
    ensure!(constraints.len() <= MAX_CONSTRAINTS, "Too many constraints");
    ensure!(
        constraints
            .iter()
            .all(|c| c.len() <= MAX_CONSTRAINT_SIZE_IN_BYTES),
        "Constraint too large"
    );
    Ok(())
}
