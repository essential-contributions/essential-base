//! Assembly for checking constraints.
//!
//! # Op Table
#![doc = essential_asm_gen::gen_constraint_ops_docs_table!()]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[doc(inline)]
pub use essential_types::Word;
#[doc(inline)]
pub use op::{Constraint as Op, *};
#[doc(inline)]
pub use opcode::{Constraint as Opcode, InvalidOpcodeError, NotEnoughBytesError};

/// Typed representation of an operation its associated data.
mod op {
    essential_asm_gen::gen_constraint_op_decls!();
    essential_asm_gen::gen_constraint_op_impls!();
    /// Provides the operation type bytes iterators.
    pub mod bytes_iter {
        essential_asm_gen::gen_constraint_op_bytes_iter!();
    }
}

/// Typed representation of the opcode, without any associated data.
pub mod opcode {
    use core::fmt;

    /// An attempt to parse a byte as an opcode failed.
    #[derive(Debug)]
    pub struct InvalidOpcodeError(pub u8);

    /// An error occurring within `Opcode::parse_op` in the case that the
    /// provided bytes iterator contains insufficient bytes for the expected
    /// associated operation data.
    #[derive(Debug)]
    pub struct NotEnoughBytesError;

    impl fmt::Display for InvalidOpcodeError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Invalid Opcode 0x{:02X}", self.0)
        }
    }

    impl fmt::Display for NotEnoughBytesError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Provided iterator did not yield enough bytes")
        }
    }

    essential_asm_gen::gen_constraint_opcode_decls!();
    essential_asm_gen::gen_constraint_opcode_impls!();
}

/// Errors that can occur while parsing ops from bytes.
#[derive(Debug)]
pub enum FromBytesError {
    /// An invalid opcode was encountered.
    InvalidOpcode(InvalidOpcodeError),
    /// The bytes iterator did not contain enough bytes for a particular operation.
    NotEnoughBytes(NotEnoughBytesError),
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
    core::iter::from_fn(move || {
        let opcode_byte = iter.next()?;
        let op_res = Opcode::try_from(opcode_byte)
            .map_err(From::from)
            .and_then(|opcode| opcode.parse_op(&mut iter).map_err(From::from));
        Some(op_res)
    })
}

/// Convert the given iterator yielding operations into and iterator yielding
/// the serialized form in bytes.
pub fn to_bytes(ops: impl IntoIterator<Item = Op>) -> impl Iterator<Item = u8> {
    ops.into_iter().flat_map(|op| op.to_bytes())
}

impl From<InvalidOpcodeError> for FromBytesError {
    fn from(err: InvalidOpcodeError) -> Self {
        Self::InvalidOpcode(err)
    }
}

impl From<NotEnoughBytesError> for FromBytesError {
    fn from(err: NotEnoughBytesError) -> Self {
        Self::NotEnoughBytes(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_roundtrip_u8() {
        for byte in 0..=core::u8::MAX {
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
            Stack::Swap.into(),
            Stack::Dup.into(),
        ];
        roundtrip(ops);
    }

    #[test]
    fn roundtrip_args_end() {
        let ops: Vec<Op> = vec![
            Stack::Swap.into(),
            Stack::Dup.into(),
            Stack::Push(0x0F0F0F0F0F0F0F0F).into(),
        ];
        roundtrip(ops);
    }

    #[test]
    fn roundtrip_args_interspersed() {
        let ops: Vec<Op> = vec![
            Stack::Push(0x1234567812345678).into(),
            Stack::Swap.into(),
            Stack::Push(0x0F0F0F0F0F0F0F0F).into(),
            Stack::Dup.into(),
            Stack::Push(0x1234567812345678).into(),
        ];
        roundtrip(ops);
    }

    #[test]
    fn roundtrip_no_args() {
        let ops: Vec<Op> = vec![
            Access::ThisAddress.into(),
            Access::ThisSetAddress.into(),
            Stack::Swap.into(),
            Stack::Dup.into(),
            Crypto::Sha256.into(),
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

    #[test]
    fn not_enough_bytes() {
        let opcode_byte = opcode::Stack::Push as u8;
        let bytes = vec![opcode_byte];
        let err = from_bytes(bytes)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_err();
        match err {
            FromBytesError::NotEnoughBytes(_) => (),
            _ => panic!("unexpected error variant"),
        }
    }
}
