//! Items related to bytecode representation for the State Read VM.

use crate::asm::{opcode::ParseOp, ToBytes, ToOpcode, TryFromBytes};

/// A memory efficient representation of a sequence of operations parsed from bytecode.
///
/// Executing certain control flow operations can require the ability to jump
/// back to a previous operation.
///
/// One simple solution might be to use a `Vec<Op>`, however it is important to
/// consider that the size of each element within a `Vec<Op>` will be equal to
/// the size of the discriminant plus the largest `Op` variant size (today, this
/// is `Push(Word)`, but this may change as new operations are added). This can
/// have memory requirement implications for programs with large numbers of ops.
///
/// To avoid this issue, we instead store the raw "packed" bytecode alongside
/// a list of indices into the bytecode representing the location of each
/// operation.
#[derive(Clone, Debug, PartialEq)]
pub struct BytecodeMapped<Op, Bytes = Vec<u8>> {
    /// The bytecode representation of a program's operations.
    bytecode: Bytes,
    /// The index of each op within the bytecode slice.
    ///
    /// Indices are guaranteed to be valid by construction and point to a valid operation.
    op_indices: Vec<usize>,
    /// Ensures that `BytecodeMapped` remains consistent for the given `Op` type.
    _op_ty: core::marker::PhantomData<Op>,
}

/// A slice into a [`BytecodeMapped`] instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BytecodeMappedSlice<'a, Op> {
    /// The full bytecode slice from the original `BytecodeMapped`.
    bytecode: &'a [u8],
    /// Some subslice into the `op_indices` of the original `BytecodeMapped`.
    op_indices: &'a [usize],
    /// Ensures that `BytecodeMapped` remains consistent for the given `Op` type.
    _op_ty: core::marker::PhantomData<Op>,
}

/// A type wrapper around `BytecodeMapped` that lazily constructs the map from
/// the given bytecode as operations are accessed.
#[derive(Debug)]
pub struct BytecodeMappedLazy<Op, I> {
    /// The `BytecodeMapped` instance that is lazily constructed.
    pub(crate) mapped: BytecodeMapped<Op>,
    /// The iterator yielding bytes.
    pub(crate) iter: I,
}

impl<Op> BytecodeMapped<Op, Vec<u8>> {
    /// Push a single operation onto the bytecode mapping.
    pub fn push_op(&mut self, op: Op)
    where
        Op: ToBytes,
    {
        self.op_indices.push(self.bytecode.len());
        self.bytecode.extend(op.to_bytes());
    }
}

impl<Op, Bytes> BytecodeMapped<Op, Bytes>
where
    Bytes: core::ops::Deref<Target = [u8]>,
{
    /// Attempt to construct a `BytecodeMapped` from an existing slice of bytes.
    ///
    /// `bytes` may be any type that dereferences to a slice of bytes, e.g.
    /// `&[u8]`, `Arc<[u8]>`, `Vec<u8>`, etc.
    pub fn try_from_bytes(bytes: Bytes) -> Result<BytecodeMapped<Op, Bytes>, Op::Error>
    where
        Op: ToOpcode + TryFromBytes,
        Op::Opcode: ParseOp<Op = Op> + TryFrom<u8>,
        Op::Error: From<<Op::Opcode as TryFrom<u8>>::Error> + From<<Op::Opcode as ParseOp>::Error>,
    {
        let bytecode = bytes.deref();
        let mut op_indices = Vec::with_capacity(bytecode.len() / std::mem::size_of::<Op>());
        let mut iter_enum = bytecode.iter().enumerate();
        while let Some((ix, &opcode_byte)) = iter_enum.next() {
            let opcode = Op::Opcode::try_from(opcode_byte)?;
            let mut op_bytes = iter_enum.by_ref().map(|(_, &byte)| byte);
            let _op = opcode.parse_op(&mut op_bytes)?;
            op_indices.push(ix);
        }
        Ok(BytecodeMapped {
            bytecode: bytes,
            op_indices,
            _op_ty: core::marker::PhantomData,
        })
    }

    /// Borrow the inner bytecode and op_indices slices and return a [`BytecodeMappedSlice`].
    pub fn as_slice(&self) -> BytecodeMappedSlice<Op> {
        BytecodeMappedSlice {
            bytecode: self.bytecode(),
            op_indices: self.op_indices(),
            _op_ty: self._op_ty,
        }
    }

    /// The inner slice of bytecode that has been mapped.
    pub fn bytecode(&self) -> &[u8] {
        self.bytecode.deref()
    }

    /// Slice the op indices from the given index.
    ///
    /// The returned slice represents the remainder of the program from the given op.
    ///
    /// Returns `None` if `start` is out of range of the `op_indices` slice.
    pub fn ops_from(&self, start: usize) -> Option<BytecodeMappedSlice<Op>> {
        Some(BytecodeMappedSlice {
            bytecode: self.bytecode(),
            op_indices: self.op_indices.get(start..)?,
            _op_ty: self._op_ty,
        })
    }

    /// The operation at the given index.
    pub fn op(&self, ix: usize) -> Option<Op>
    where
        Op: TryFromBytes,
    {
        let slice = self.ops_from(ix)?;
        slice.ops().next()
    }

    /// An iterator yielding all mapped operations.
    pub fn ops(&self) -> impl '_ + Iterator<Item = Op>
    where
        Op: TryFromBytes,
    {
        expect_ops_from_indices(self.bytecode(), self.op_indices.iter().copied())
    }
}

impl<Op, Bytes> BytecodeMapped<Op, Bytes> {
    /// The slice of operation indices within the mapped bytecode.
    pub fn op_indices(&self) -> &[usize] {
        &self.op_indices
    }
}

impl<'a, Op> BytecodeMappedSlice<'a, Op> {
    /// The slice of operation indices within the mapped bytecode.
    pub fn op_indices(self) -> &'a [usize] {
        self.op_indices
    }

    /// An iterator yielding all mapped operations represented by this slice.
    pub fn ops(self) -> impl 'a + Iterator<Item = Op>
    where
        Op: TryFromBytes,
    {
        expect_ops_from_indices(self.bytecode, self.op_indices.iter().copied())
    }
}

impl<Op, I> BytecodeMappedLazy<Op, I> {
    /// Construct the `BytecodeMappedLazy` from its bytecode iterator.
    pub fn new<J>(bytes: J) -> Self
    where
        J: IntoIterator<IntoIter = I>,
        I: Iterator<Item = u8>,
    {
        let iter = bytes.into_iter();
        let (min, _) = iter.size_hint();
        let mapped = BytecodeMapped {
            bytecode: Vec::with_capacity(min),
            op_indices: Vec::with_capacity(min),
            _op_ty: core::marker::PhantomData,
        };
        Self { mapped, iter }
    }
}

/// Manually implement `Default` to avoid requiring that `Op: Default` as is
/// assumed by `derive(Default)`.
impl<Op> Default for BytecodeMapped<Op> {
    fn default() -> Self {
        BytecodeMapped {
            bytecode: Default::default(),
            op_indices: Default::default(),
            _op_ty: Default::default(),
        }
    }
}

// Allow for collecting a `BytecodeMapped` from an iterator over `Op`s.
impl<Op> FromIterator<Op> for BytecodeMapped<Op>
where
    Op: ToBytes,
{
    fn from_iter<T: IntoIterator<Item = Op>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (min, _) = iter.size_hint();
        let mut mapped = BytecodeMapped {
            bytecode: Vec::with_capacity(min),
            op_indices: Vec::with_capacity(min),
            _op_ty: core::marker::PhantomData,
        };
        iter.for_each(|op| mapped.push_op(op));
        mapped
    }
}

/// Allow for taking ownership over and mapping an existing `Vec<u8>`.
impl<Op> TryFrom<Vec<u8>> for BytecodeMapped<Op>
where
    Op: ToOpcode + TryFromBytes,
    Op::Opcode: ParseOp<Op = Op> + TryFrom<u8>,
    Op::Error: From<<Op::Opcode as TryFrom<u8>>::Error> + From<<Op::Opcode as ParseOp>::Error>,
{
    type Error = Op::Error;
    fn try_from(bytecode: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytecode)
    }
}

/// Allow for consuming and mapping an existing `&[u8]`.
impl<'a, Op> TryFrom<&'a [u8]> for BytecodeMapped<Op, &'a [u8]>
where
    Op: ToOpcode + TryFromBytes,
    Op::Opcode: ParseOp<Op = Op> + TryFrom<u8>,
    Op::Error: From<<Op::Opcode as TryFrom<u8>>::Error> + From<<Op::Opcode as ParseOp>::Error>,
{
    type Error = Op::Error;
    fn try_from(bytecode: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytecode)
    }
}

/// Given a bytecode slice and an operation mapping that is assumed to have been
/// previously validated, produce an iterator yielding all associated operations.
fn expect_ops_from_indices<'a, Op>(
    bytecode: &'a [u8],
    op_indices: impl 'a + IntoIterator<Item = usize>,
) -> impl 'a + Iterator<Item = Op>
where
    Op: TryFromBytes,
{
    const EXPECT_MSG: &str = "validated upon construction";
    op_indices.into_iter().map(|ix| {
        let mut bytes = bytecode[ix..].iter().copied();
        Op::try_from_bytes(&mut bytes)
            .expect(EXPECT_MSG)
            .expect(EXPECT_MSG)
    })
}
