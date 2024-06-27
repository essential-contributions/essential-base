//! Core logic for validating [`Predicate`][crate::types::predicate::Predicate]s,
//! [`Solution`][crate::types::solution::Solution]s and
//! [`SolutionData`][crate::types::solution::SolutionData] against associated predicates.
//!
//! Typical usage is to first validate predicates and solutions independently in
//! full prior to validating against one another with `solution::check_predicates`.
//!
//! ## Predicate Validation
//!
//! - [`predicate::check_signed_contract`] validates a signed contract.
//! - [`predicate::check_contract`] validates a contract.
//! - [`predicate::check`] validate an individual predicate.
//! - [`predicate::check_slots`] validate an predicate's slots.
//! - [`predicate::check_directive`] validate an predicate's directive.
//! - [`predicate::check_state_reads`] validate an predicate's state read bytecode.
//! - [`predicate::check_constraints`] validate an predicate's constraint bytecode.
//!
//! ## Solution Validation
//!
//! - [`solution::check`] validates an unsigned solution.
//! - [`solution::check_data`] validates a solution's data slice.
//! - [`solution::check_state_mutations`] validates a solution's state mutation slice.
//!
//! ## Solution + Predicate Validation
//!
//! - [`solution::check_predicates`] validates a solution's data against their associated predicates.
//! - [`solution::check_predicate`] validates a single solution data against an associated predicate.
//! - [`solution::check_predicate_constraints`] the predicate constraint checking part of solution
//!   data validation.
//! - [`solution::check_decision_variable_lengths`] checks the expected number of
//!   decision variables.

#![deny(missing_docs)]
#![deny(unsafe_code)]

#[doc(inline)]
pub use essential_constraint_vm as constraint_vm;
#[doc(inline)]
pub use essential_sign as sign;
#[doc(inline)]
pub use essential_state_read_vm as state_read_vm;
#[doc(inline)]
pub use essential_types as types;

pub mod predicate;
pub mod solution;
