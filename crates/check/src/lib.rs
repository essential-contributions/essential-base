//! Core logic for validating [`Predicate`][crate::types::predicate::Predicate]s,
//! [`SolutionSet`][crate::types::solution::SolutionSet]s and
//! [`Solution`][crate::types::solution::Solution]s against their associated predicates.
//!
//! Typical usage is to first validate predicates and solutions independently in
//! full prior to validating against one another with `solution::check_set_predicates`.
//!
//! ## Predicate Validation
//!
//! - [`predicate::check_signed_contract`] validates a signed contract.
//! - [`predicate::check_contract`] validates a contract.
//! - [`predicate::check`] validate an individual predicate.
//!
//! ## Solution Validation
//!
//! - [`solution::check_set`] validates a solution set.
//! - [`solution::check_solutions`] validates a solution set's `solutions` slice.
//! - [`solution::check_set_state_mutations`] validates a solution's state mutation slice.
//!
//! ## Solution + Predicate Validation
//!
//! - [`solution::check_set_predicates`] validates a set of solutions against their associated predicates.
//! - [`solution::check_predicate`] validates a single solution against its associated predicate.

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
