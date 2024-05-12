//! Assembly for checking constraints.
//!
//! # Op Table
#![doc = essential_asm_gen::gen_constraint_ops_docs_table!()]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use core::fmt;
#[doc(inline)]
pub use essential_types::Word;
#[doc(inline)]
pub use op::{Constraint as Op, *};
#[doc(inline)]
pub use opcode::{Constraint as Opcode, InvalidOpcodeError, NotEnoughBytesError};

/// Typed representation of an operation its associated data.
mod op {
    /// Operation types that may be converted to their serialized form in bytes.
    pub trait ToBytes {
        /// The iterator yielding bytes.
        type Bytes: IntoIterator<Item = u8>;
        /// Convert the operation to its serialized form in bytes.
        fn to_bytes(&self) -> Self::Bytes;
    }

    /// Allows for converting an `Op` into its associated `Opcode`.
    pub trait ToOpcode {
        /// The associated `Opcode` type.
        type Opcode;
        /// The `opcode` associated with this operation.
        fn to_opcode(&self) -> Self::Opcode;
    }

    /// Operation types that may be parsed from a bytecode representation.
    pub trait TryFromBytes: Sized {
        /// Represents any error that might occur while parsing an op from bytes.
        type Error: core::fmt::Debug + core::fmt::Display;
        /// Parse a single operation from the given iterator yielding bytes.
        ///
        /// Returns `None` in the case that the given iterator is empty.
        fn try_from_bytes(
            bytes: &mut impl Iterator<Item = u8>,
        ) -> Option<Result<Self, Self::Error>>;
    }

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

    /// Parse the operation associated with the opcode.
    pub trait ParseOp {
        /// The operation associated with the opcode.
        type Op;
        /// Any error that might occur while parsing.
        type Error: core::fmt::Debug + core::fmt::Display;
        /// Attempt to parse the operation associated with the opcode from the given bytes.
        ///
        /// Only consumes the bytes necessary to construct any associated data.
        ///
        /// Returns an error in the case that the given `bytes` iterator
        /// contains insufficient bytes to parse the op.
        fn parse_op(&self, bytes: &mut impl Iterator<Item = u8>) -> Result<Self::Op, Self::Error>;
    }

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

    impl std::error::Error for InvalidOpcodeError {}

    impl std::error::Error for NotEnoughBytesError {}

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

impl fmt::Display for FromBytesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("failed to parse ops from bytes: ")?;
        match self {
            Self::InvalidOpcode(err) => err.fmt(f),
            Self::NotEnoughBytes(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for FromBytesError {}

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
