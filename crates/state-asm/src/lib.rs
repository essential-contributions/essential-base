//! Assembly for state read operations.
//!
//! # Op Table
#![doc = essential_asm_gen::gen_ops_docs_table!()]
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[doc(inline)]
pub use essential_constraint_asm::{FromBytesError, Word};
#[doc(inline)]
pub use op::{StateRead as Op, *};
#[doc(inline)]
pub use opcode::{InvalidOpcodeError, NotEnoughBytesError, StateRead as Opcode};

/// Typed representation of an operation its associated data.
mod op {
    pub use essential_constraint_asm::{
        Access, Alu, Constraint, Crypto, Pred, Stack, ToBytes, ToOpcode, TryFromBytes,
    };
    essential_asm_gen::gen_state_read_op_decls!();
    essential_asm_gen::gen_state_read_op_impls!();
    /// Provides the operation type bytes iterators.
    pub mod bytes_iter {
        pub use essential_constraint_asm::bytes_iter::*;
        essential_asm_gen::gen_state_read_op_bytes_iter!();
    }
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
    core::iter::from_fn(move || Op::try_from_bytes(&mut iter))
}

/// Convert the given iterator yielding operations into and iterator yielding
/// the serialized form in bytes.
pub fn to_bytes(ops: impl IntoIterator<Item = Op>) -> impl Iterator<Item = u8> {
    ops.into_iter().flat_map(|op| op.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_roundtrip_u8() {
        for byte in 0..=std::u8::MAX {
            if let Ok(opcode) = Opcode::try_from(byte) {
                println!("{byte:02X}: {opcode:?}");
                assert_eq!(u8::from(opcode), byte);
            }
        }
    }

    fn roundtrip(ops: Vec<Op>) {
        assert!(!ops.is_empty());
        let bytes: Vec<_> = to_bytes(ops.iter().cloned()).collect();
        assert_eq!(
            ops,
            from_bytes(bytes).collect::<Result<Vec<_>, _>>().unwrap()
        );
    }

    #[test]
    fn roundtrip_args_start() {
        let ops: Vec<Op> = vec![
            Stack::Push(0x1234567812345678).into(),
            Stack::Push(0x0F0F0F0F0F0F0F0F).into(),
            Memory::Alloc.into(),
            Memory::Free.into(),
        ];
        roundtrip(ops);
    }

    #[test]
    #[allow(clippy::useless_conversion)]
    fn roundtrip_args_end() {
        let ops: Vec<Op> = vec![
            StateRead::WordRange.into(),
            StateRead::WordRangeExtern.into(),
            Stack::Push(0x0F0F0F0F0F0F0F0F).into(),
        ];
        roundtrip(ops);
    }

    #[test]
    fn roundtrip_args_interspersed() {
        let ops: Vec<Op> = vec![
            Stack::Push(0x1234567812345678).into(),
            ControlFlow::Jump.into(),
            Stack::Push(0x0F0F0F0F0F0F0F0F).into(),
            ControlFlow::Halt.into(),
            Stack::Push(0x1234567812345678).into(),
        ];
        roundtrip(ops);
    }

    #[test]
    fn roundtrip_no_args() {
        let ops: Vec<Op> = vec![
            Memory::Store.into(),
            Access::ThisAddress.into(),
            Memory::Load.into(),
            Access::ThisSetAddress.into(),
            Memory::Capacity.into(),
        ];
        roundtrip(ops);
    }

    fn expect_invalid_opcode(opcode_byte: u8) {
        let bytes = vec![opcode_byte];
        let err = from_bytes(bytes)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_err();
        match err {
            FromBytesError::InvalidOpcode(InvalidOpcodeError(byte)) => {
                assert_eq!(byte, opcode_byte)
            }
            _ => panic!("unexpected error variant"),
        }
    }

    #[test]
    fn invalid_opcode_0x00() {
        let opcode_byte = 0x00;
        expect_invalid_opcode(opcode_byte);
    }

    #[test]
    fn invalid_opcode_0xff() {
        let opcode_byte = 0xFF;
        expect_invalid_opcode(opcode_byte);
    }
}
