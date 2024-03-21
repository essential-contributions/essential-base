//! Assembly for checking constraints.
//!
//! # Op Table
#![doc = essential_asm_gen::gen_constraint_ops_docs_table!()]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[doc(inline)]
pub use op::{Constraint as Op, *};
#[doc(inline)]
pub use opcode::Constraint as Opcode;

/// Typed representation of an operation its associated data.
mod op {
    essential_asm_gen::gen_constraint_op_decls!();
}

/// Typed representation of the opcode, without any associated data.
pub mod opcode {
    essential_asm_gen::gen_constraint_opcode_decls!();
}
