//! Items related to the validation of [`Predicate`]s.

use crate::{
    sign::secp256k1,
    types::{
        predicate::{Directive, Predicate},
        ConstraintBytecode, StateReadBytecode,
    },
};
#[cfg(feature = "tracing")]
use essential_hash::contract_addr;
use essential_types::contract;
use thiserror::Error;

/// [`check_signed_contract`] error.
#[derive(Debug, Error)]
pub enum InvalidSignedContract {
    /// Failed to validate the signature over the contract.
    #[error("invalid signature: {0}")]
    Signature(#[from] secp256k1::Error),
    /// The contract was invalid.
    #[error("invalid contract: {0}")]
    Set(#[from] InvalidContract),
}

/// [`check_contract`] error.
#[derive(Debug, Error)]
pub enum InvalidContract {
    /// The number of predicates in the contract exceeds the limit.
    #[error("the number of predicates ({0}) exceeds the limit ({MAX_PREDICATES})")]
    TooManyPredicates(usize),
    /// The predicate at the given index was invalid.
    #[error("predicate at index {0} is invalid: {1}")]
    Predicate(usize, InvalidPredicate),
}

/// [`check`] error indicating part of an predicate was invalid.
#[derive(Debug, Error)]
pub enum InvalidPredicate {
    /// The predicate's slots are invalid.
    #[error("invalid slots: {0}")]
    Slots(#[from] InvalidSlots),
    /// The predicate's directive is invalid.
    #[error("invalid directive: {0}")]
    Directive(#[from] InvalidDirective),
    /// The predicate's state reads are invalid.
    #[error("invalid state reads: {0}")]
    StateReads(#[from] InvalidStateReads),
    /// The predicate's constraints are invalid.
    #[error("invalid constraints: {0}")]
    Constraints(#[from] InvalidConstraints),
}

/// [`check_slots`] error.
#[derive(Debug, Error)]
pub enum InvalidSlots {
    /// The predicate expects too many decision variables.
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

/// Maximum number of predicates in a contract.
pub const MAX_PREDICATES: usize = 100;
/// Maximum number of state read programs of an predicate.
pub const MAX_STATE_READS: usize = 100;
/// Maximum size of state read programs of an predicate in bytes.
pub const MAX_STATE_READ_SIZE_IN_BYTES: usize = 10_000;
/// Maximum number of constraint check programs of an predicate.
pub const MAX_CONSTRAINTS: usize = 100;
/// Maximum size of constraint check programs of an predicate in bytes.
pub const MAX_CONSTRAINT_SIZE_IN_BYTES: usize = 10_000;
/// Maximum number of decision variables of the slots of an predicate.
pub const MAX_DECISION_VARIABLES: u32 = 100;
/// Maximum number of state slots of an predicate.
pub const MAX_NUM_STATE_SLOTS: usize = 1000;
/// Maximum length of state slots of an predicate.
pub const MAX_STATE_LEN: u32 = 1000;
/// Maximum size of directive of an predicate.
pub const MAX_DIRECTIVE_SIZE: usize = 1000;

/// Validate a signed contract of predicates.
///
/// Verifies the signature and then validates the contract.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(addr = %contract_addr::from_contract(&signed_contract.contract)), err))]
pub fn check_signed_contract(
    signed_contract: &contract::SignedContract,
) -> Result<(), InvalidSignedContract> {
    essential_sign::contract::verify(signed_contract)?;
    check_contract(signed_contract.contract.as_ref())?;
    Ok(())
}

/// Validate a contract of predicates.
///
/// Checks the size of the contract and then validates each predicate.
pub fn check_contract(predicates: &[Predicate]) -> Result<(), InvalidContract> {
    if predicates.len() > MAX_PREDICATES {
        return Err(InvalidContract::TooManyPredicates(predicates.len()));
    }
    for (ix, predicate) in predicates.iter().enumerate() {
        check(predicate).map_err(|e| InvalidContract::Predicate(ix, e))?;
    }
    Ok(())
}

/// Validate a single predicate.
///
/// Validates the slots, directive, state reads, and constraints.
pub fn check(predicate: &Predicate) -> Result<(), InvalidPredicate> {
    check_directive(&predicate.directive)?;
    check_state_reads(&predicate.state_read)?;
    check_constraints(&predicate.constraints)?;
    Ok(())
}

/// Validate an predicate's directive.
pub fn check_directive(directive: &Directive) -> Result<(), InvalidDirective> {
    if let Directive::Maximize(program) | Directive::Minimize(program) = directive {
        if program.len() > MAX_DIRECTIVE_SIZE {
            return Err(InvalidDirective::TooManyBytes(program.len()));
        }
    }
    Ok(())
}

/// Validate an predicate's state read bytecode.
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

/// Validate an predicate's constraint bytecode.
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
