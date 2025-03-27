//! The `OpAccess` trait declaration and its implementations.

use crate::{asm::TryFromBytes, bytecode::BytecodeMapped};

/// Types that provide access to operations.
///
/// Implementations are included for `&[Op]` and [`BytecodeMapped`].
pub trait OpAccess {
    /// The operation type being accessed.
    type Op;
    /// Any error that might occur during access.
    type Error: core::fmt::Debug + core::fmt::Display;
    /// Access the operation at the given index.
    ///
    /// Mutable access to self is required in case operations are lazily parsed.
    ///
    /// Any implementation should ensure the same index always returns the same operation.
    fn op_access(&self, index: usize) -> Option<Result<Self::Op, Self::Error>>;
}

impl<Op> OpAccess for &[Op]
where
    Op: Clone,
{
    type Op = Op;
    type Error = core::convert::Infallible;
    fn op_access(&self, index: usize) -> Option<Result<Self::Op, Self::Error>> {
        self.get(index).cloned().map(Ok)
    }
}

impl<Op, Bytes> OpAccess for &BytecodeMapped<Op, Bytes>
where
    Op: TryFromBytes,
    Bytes: core::ops::Deref<Target = [u8]>,
{
    type Op = Op;
    type Error = core::convert::Infallible;
    fn op_access(&self, index: usize) -> Option<Result<Self::Op, Self::Error>> {
        self.op(index).map(Ok)
    }
}
