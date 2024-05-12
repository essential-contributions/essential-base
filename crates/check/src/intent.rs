//! Items related to the validation of [`Intent`]s.

use crate::{
    sign::{secp256k1, verify},
    types::{
        intent::{Directive, Intent},
        slots::{state_len, Slots},
        ConstraintBytecode, Signed, StateReadBytecode,
    },
};
use thiserror::Error;

/// [`check_signed_set`] error.
#[derive(Debug, Error)]
pub enum InvalidSignedSet {
    /// Failed to validate the signature over the set.
    #[error("invalid signature: {0}")]
    Signature(#[from] secp256k1::Error),
    /// The intent set was invalid.
    #[error("invalid set: {0}")]
    Set(#[from] InvalidSet),
}

/// [`check_set`] error.
#[derive(Debug, Error)]
pub enum InvalidSet {
    /// The number of intents in the set exceeds the limit.
    #[error("the number of intents ({0}) exceeds the limit ({MAX_INTENTS})")]
    TooManyIntents(usize),
    /// The intent at the given index was invalid.
    #[error("intent at index {0} is invalid: {1}")]
    Intent(usize, InvalidIntent),
}

/// [`check`] error indicating part of an intent was invalid.
#[derive(Debug, Error)]
pub enum InvalidIntent {
    /// The intent's slots are invalid.
    #[error("invalid slots: {0}")]
    Slots(#[from] InvalidSlots),
    /// The intent's directive is invalid.
    #[error("invalid directive: {0}")]
    Directive(#[from] InvalidDirective),
    /// The intent's state reads are invalid.
    #[error("invalid state reads: {0}")]
    StateReads(#[from] InvalidStateReads),
    /// The intent's constraints are invalid.
    #[error("invalid constraints: {0}")]
    Constraints(#[from] InvalidConstraints),
}

/// [`check_slots`] error.
#[derive(Debug, Error)]
pub enum InvalidSlots {
    /// The intent expects too many decision variables.
    #[error("the number of decision vars ({0}) exceeds the limit ({MAX_DECISION_VARIABLES})")]
    TooManyDecisionVariables(u32),
    /// The number of state slots exceeds the limit.
    #[error("the number of state slots ({0}) exceeds the limit ({MAX_NUM_STATE_SLOTS})")]
    TooManyStateSlots(usize),
    /// The total length of all state slots exceeds the limit.
    ///
    /// `None` in the case that the length exceeds `u32::MAX`.
    #[error("the total length of all state slots ({0:?}) exceeds the limit ({MAX_STATE_LEN})")]
    StateSlotLengthExceedsLimit(Option<u32>),
}

/// [`check_directive`] error.
#[derive(Debug, Error)]
pub enum InvalidDirective {
    /// The length of the bytecode exceeds the limit.
    #[error("the length of the bytecode ({0}) exceeds the limit ({MAX_DIRECTIVE_SIZE})")]
    TooManyBytes(usize),
}

/// [`check_state_reads`] error.
#[derive(Debug, Error)]
pub enum InvalidStateReads {
    /// The number of state reads exceeds the limit.
    #[error("the number of state reads ({0}) exceeds the limit ({MAX_STATE_READS})")]
    TooMany(usize),
    /// The state read at the given index failed to validate.
    #[error("state read at index {0} failed to validate: {1}")]
    StateRead(usize, InvalidStateRead),
}

/// [`check_state_read`] error.
#[derive(Debug, Error)]
pub enum InvalidStateRead {
    /// The length of the bytecode exceeds the limit.
    #[error("the length of the bytecode ({0}) exceeds the limit ({MAX_STATE_READ_SIZE_IN_BYTES}")]
    TooManyBytes(usize),
}

/// [`check_constraints`] error.
#[derive(Debug, Error)]
pub enum InvalidConstraints {
    /// The number of constraints exceeds the limit.
    #[error("the number of constraints ({0}) exceeds the limit ({MAX_CONSTRAINTS})")]
    TooManyConstraints(usize),
    /// The constraint at the given index failed to validate.
    #[error("constraint at index {0} failed to validate: {1}")]
    Constraint(usize, InvalidConstraint),
}

/// [`check_constraint`] error.
#[derive(Debug, Error)]
pub enum InvalidConstraint {
    /// The length of the bytecode exceeds the limit.
    #[error("the length of the bytecode ({0}) exceeds the limit ({MAX_CONSTRAINT_SIZE_IN_BYTES}")]
    TooManyBytes(usize),
}

/// Maximum number of intents in a set.
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
pub fn check_signed_set(intents: &Signed<Vec<Intent>>) -> Result<(), InvalidSignedSet> {
    verify(intents)?;
    check_set(&intents.data)?;
    Ok(())
}

/// Validate a set of intents.
///
/// Checks the size of the set and then validates each intent.
pub fn check_set(intents: &[Intent]) -> Result<(), InvalidSet> {
    if intents.len() > MAX_INTENTS {
        return Err(InvalidSet::TooManyIntents(intents.len()));
    }
    for (ix, intent) in intents.iter().enumerate() {
        check(intent).map_err(|e| InvalidSet::Intent(ix, e))?;
    }
    Ok(())
}

/// Validate a single intent.
///
/// Validates the slots, directive, state reads, and constraints.
pub fn check(intent: &Intent) -> Result<(), InvalidIntent> {
    check_slots(&intent.slots)?;
    check_directive(&intent.directive)?;
    check_state_reads(&intent.state_read)?;
    check_constraints(&intent.constraints)?;
    Ok(())
}

/// Validate an intent's slots.
///
/// Checks the number of decision variables, state slots and the total state length in words.
pub fn check_slots(slots: &Slots) -> Result<(), InvalidSlots> {
    if slots.decision_variables > MAX_DECISION_VARIABLES {
        return Err(InvalidSlots::TooManyDecisionVariables(
            slots.decision_variables,
        ));
    }
    if slots.state.len() > MAX_NUM_STATE_SLOTS {
        return Err(InvalidSlots::TooManyStateSlots(slots.state.len()));
    }
    match state_len(&slots.state) {
        None => Err(InvalidSlots::StateSlotLengthExceedsLimit(None)),
        Some(len) if len > MAX_STATE_LEN => {
            Err(InvalidSlots::StateSlotLengthExceedsLimit(Some(len)))
        }
        _ => Ok(()),
    }
}

/// Validate an intent's directive.
pub fn check_directive(directive: &Directive) -> Result<(), InvalidDirective> {
    if let Directive::Maximize(program) | Directive::Minimize(program) = directive {
        if program.len() > MAX_DIRECTIVE_SIZE {
            return Err(InvalidDirective::TooManyBytes(program.len()));
        }
    }
    Ok(())
}

/// Validate an intent's state read bytecode.
pub fn check_state_reads(state_reads: &[StateReadBytecode]) -> Result<(), InvalidStateReads> {
    if state_reads.len() > MAX_STATE_READS {
        return Err(InvalidStateReads::TooMany(state_reads.len()));
    }
    for (ix, state_read) in state_reads.iter().enumerate() {
        check_state_read(state_read).map_err(|e| InvalidStateReads::StateRead(ix, e))?;
    }
    Ok(())
}

/// Validate a single state read bytecode slice.
pub fn check_state_read(state_read: &[u8]) -> Result<(), InvalidStateRead> {
    if state_read.len() > MAX_STATE_READ_SIZE_IN_BYTES {
        return Err(InvalidStateRead::TooManyBytes(state_read.len()));
    }
    Ok(())
}

/// Validate an intent's constraint bytecode.
pub fn check_constraints(constraints: &[ConstraintBytecode]) -> Result<(), InvalidConstraints> {
    if constraints.len() > MAX_CONSTRAINTS {
        return Err(InvalidConstraints::TooManyConstraints(constraints.len()));
    }
    for (ix, constraint) in constraints.iter().enumerate() {
        check_constraint(constraint).map_err(|e| InvalidConstraints::Constraint(ix, e))?;
    }
    Ok(())
}

/// Validate a single constraint bytecode slice.
pub fn check_constraint(constraint: &[u8]) -> Result<(), InvalidConstraint> {
    if constraint.len() > MAX_CONSTRAINT_SIZE_IN_BYTES {
        return Err(InvalidConstraint::TooManyBytes(constraint.len()));
    }
    Ok(())
}
