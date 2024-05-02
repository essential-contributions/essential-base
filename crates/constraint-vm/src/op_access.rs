//! The `OpAccess` trait declaration and its implementations.

use crate::{
    asm::{ToBytes, TryFromBytes},
    BytecodeMapped, BytecodeMappedLazy,
};

/// Types that provide access to operations.
///
/// Implementations are included for `&[Op]`, [`BytecodeMapped`] and [`BytecodeMappedLazy`].
pub trait OpAccess {
    /// The operation type being accessed.
    type Op;
    /// Any error that might occur during access.
    type Error: std::error::Error;
    /// Access the operation at the given index.
    ///
    /// Mutable access to self is required in case operations are lazily parsed.
    ///
    /// Any implementation should ensure the same index always returns the same operation.
    fn op_access(&mut self, index: usize) -> Option<Result<Self::Op, Self::Error>>;
}

impl<'a, Op> OpAccess for &'a [Op]
where
    Op: Clone,
{
    type Op = Op;
    type Error = core::convert::Infallible;
    fn op_access(&mut self, index: usize) -> Option<Result<Self::Op, Self::Error>> {
        self.get(index).cloned().map(Ok)
    }
}

impl<'a, Op, Bytes> OpAccess for &'a BytecodeMapped<Op, Bytes>
where
    Op: TryFromBytes,
    Bytes: core::ops::Deref<Target = [u8]>,
{
    type Op = Op;
    type Error = core::convert::Infallible;
    fn op_access(&mut self, index: usize) -> Option<Result<Self::Op, Self::Error>> {
        self.op(index).map(Ok)
    }
}

impl<Op, I> OpAccess for BytecodeMappedLazy<Op, I>
where
    Op: ToBytes + TryFromBytes,
    I: Iterator<Item = u8>,
{
    type Op = Op;
    type Error = Op::Error;
    fn op_access(&mut self, index: usize) -> Option<Result<Op, Self::Error>> {
        loop {
            match self.mapped.op(index) {
                Some(op) => return Some(Ok(op)),
                None => match Op::try_from_bytes(&mut self.iter)? {
                    Err(err) => return Some(Err(err)),
                    Ok(op) => self.mapped.push_op(op),
                },
            }
        }
    }
}
