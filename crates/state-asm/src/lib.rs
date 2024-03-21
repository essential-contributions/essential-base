//! Assembly for state read operations.
//!
//! # Op Table
#![doc = essential_asm_gen::gen_ops_docs_table!()]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[doc(inline)]
pub use essential_constraint_asm::{FromBytesError, Word};
#[doc(inline)]
pub use op::{StateRead as Op, *};
#[doc(inline)]
pub use opcode::StateRead as Opcode;

/// Typed representation of an operation its associated data.
mod op {
    pub use essential_constraint_asm::{Access, Alu, Constraint, Crypto, Pred, Stack};
    essential_asm_gen::gen_state_read_op_decls!();
    essential_asm_gen::gen_state_read_op_impls!();
}

/// Typed representation of the opcode, without any associated data.
pub mod opcode {
    pub use essential_constraint_asm::opcode::*;
    essential_asm_gen::gen_state_read_opcode_decls!();
    essential_asm_gen::gen_state_read_opcode_impls!();
}

/// Parse operations from the given iterator yielding bytes.
///
/// Returns an iterator yielding `Op` results, erroring in the case that an
/// invalid opcode is encountered or the iterator contains insufficient bytes
/// for an operation.
pub fn from_bytes(
    bytes: impl IntoIterator<Item = u8>,
) -> impl Iterator<Item = Result<Op, FromBytesError>> {
    let mut iter = bytes.into_iter();
    std::iter::from_fn(move || {
        let opcode_byte = iter.next()?;
        let op_res = Opcode::try_from(opcode_byte)
            .map_err(|_| FromBytesError::InvalidOpcode(opcode_byte))
            .and_then(|opcode| {
                opcode
                    .parse_op(&mut iter)
                    .map_err(|_| FromBytesError::InsufficientArgBytes)
            });
        Some(op_res)
    })
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
