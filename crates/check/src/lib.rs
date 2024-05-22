//! Core logic for validating [`Intent`][crate::types::intent::Intent]s,
//! [`Solution`][crate::types::solution::Solution]s and
//! [`SolutionData`][crate::types::solution::SolutionData] against associated intents.
//!
//! Typical usage is to first validate intents and solutions independently in
//! full prior to validating against one another with `solution::check_intents`.
//!
//! ## Intent Validation
//!
//! - [`intent::check_signed_set`] validates a signed set of intents.
//! - [`intent::check_set`] validates a set of intents.
//! - [`intent::check`] validate an individual intent.
//! - [`intent::check_slots`] validate an intent's slots.
//! - [`intent::check_directive`] validate an intent's directive.
//! - [`intent::check_state_reads`] validate an intent's state read bytecode.
//! - [`intent::check_constraints`] validate an intent's constraint bytecode.
//!
//! ## Solution Validation
//!
//! - [`solution::check`] validates an unsigned solution.
//! - [`solution::check_data`] validates a solution's data slice.
//! - [`solution::check_state_mutations`] validates a solution's state mutation slice.
//!
//! ## Solution + Intent Validation
//!
//! - [`solution::check_intents`] validates a solution's data against their associated intents.
//! - [`solution::check_intent`] validates a single solution data against an associated intent.
//! - [`solution::check_intent_constraints`] the intent constraint checking part of solution
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

pub mod intent;
pub mod solution;
