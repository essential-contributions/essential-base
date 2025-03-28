//! Items related to the validation of [`Predicate`]s.

use crate::sign::secp256k1;
#[cfg(feature = "tracing")]
use essential_hash::content_addr;
use essential_types::{contract, predicate::Predicate, Program};
use essential_vm::asm::{self, ToOpcode};
use thiserror::Error;

#[cfg(test)]
mod tests;

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

/// [`check`] error.
#[derive(Debug, Error)]
pub enum InvalidPredicate {
    /// The number of nodes in the predicate exceeds the limit.
    #[error(
        "the number of nodes ({0}) exceeds the limit ({})",
        Predicate::MAX_NODES
    )]
    TooManyNodes(usize),
    /// The number of edges in the predicate exceeds the limit.
    #[error(
        "the number of edges ({0}) exceeds the limit ({})",
        Predicate::MAX_EDGES
    )]
    TooManyEdges(usize),
}

/// Maximum number of predicates in a contract.
pub const MAX_PREDICATES: usize = 100;

/// Validate a signed contract of predicates.
///
/// Verifies the signature and then validates the contract.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(addr = %content_addr(&signed_contract.contract)), err))]
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
/// Validates the slots, state reads, and constraints.
pub fn check(predicate: &Predicate) -> Result<(), InvalidPredicate> {
    if predicate.nodes.len() > Predicate::MAX_NODES.into() {
        return Err(InvalidPredicate::TooManyNodes(predicate.nodes.len()));
    }
    if predicate.edges.len() > Predicate::MAX_EDGES.into() {
        return Err(InvalidPredicate::TooManyEdges(predicate.edges.len()));
    }
    // FIXME: Update this to check DAG validity.
    Ok(())
}

/// Check if the predicate has a state read for the post state key range.
///
/// This avoids decoding the bytes of the predicate and instead checks the
/// opcodes bytes directly.
///
/// Push also needs to be checked as it can introduce arbitrary bytes.
///
/// This is short circuiting and will return true on the first match.
pub fn check_program_for_post_state_read(program: &Program) -> bool {
    // PostKeyRange byte
    let key: u8 = asm::Op::StateRead(asm::StateRead::PostKeyRange)
        .to_opcode()
        .into();

    // PostKeyRangeExtern byte
    let key_extern: u8 = asm::Op::StateRead(asm::StateRead::PostKeyRangeExtern)
        .to_opcode()
        .into();

    // Push byte
    let push: u8 = asm::Op::Stack(asm::Stack::Push(0)).to_opcode().into();

    let mut iter = program.0.iter();

    // Iterate over the program and check for the op codes.
    loop {
        match iter.next() {
            // Found a post state read so return true.
            Some(op) if *op == key || *op == key_extern => return true,
            // Found a push so ignore the next 8 bytes
            Some(op) if *op == push => iter.by_ref().take(8).for_each(|_| ()),
            // Any other op code
            Some(_) => (),
            // Finished and didn't find anything
            None => return false,
        }
    }
}
