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

#![deny(missing_docs)]
#![deny(unsafe_code)]

#[doc(inline)]
pub use essential_sign as sign;
#[doc(inline)]
pub use essential_types as types;
#[doc(inline)]
pub use essential_vm as vm;

pub mod predicate;
pub mod solution;
