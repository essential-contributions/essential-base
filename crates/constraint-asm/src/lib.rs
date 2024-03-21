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
    essential_asm_gen::gen_constraint_op_impls!();
}

/// Typed representation of the opcode, without any associated data.
pub mod opcode {
    essential_asm_gen::gen_constraint_opcode_decls!();
    essential_asm_gen::gen_constraint_opcode_impls!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_roundtrip_u8() {
        for byte in 0..=std::u8::MAX {
            if let Ok(opcode) = Opcode::try_from(byte) {
                println!("{byte:02X}: {opcode:?}");
                assert_eq!(u8::from(opcode), byte);
            }
        }
    }
}
