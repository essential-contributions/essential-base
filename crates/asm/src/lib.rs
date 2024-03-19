//! # Overview
//!
//! The following is an overview of all operations. For more details about an
//! individual operation, follow the link to the expanded docs.
#![doc = essential_asm_gen::asm_table_docs!()]
//! # Constraint Execution
//!
//! The [`Constraint`] type declares all operations that are permitted within
//! constraint execution. Notably, this does not include any control flow or
//! panicking operations.
//!
//! Constraint execution is **total**. That is, it is guaranteed to complete
//! execution within some finite amount of time.
//!
//! # State Read Execution
//!
//! The top-level [`Op`] type declares all operations that available to state
//! read execution. State read execution is a superset of constraint execution,
//! and includes control flow, memory and state access operations.

// Generate the ASM declarations and implementations from the ASM YAML spec.
essential_asm_gen::asm_gen!();
