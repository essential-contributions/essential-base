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
pub struct BytecodeMapped<Op> {
    /// The bytecode representation of a program's operations.
    bytecode: Vec<u8>,
    /// The index of each op within the bytecode slice.
    ///
    /// Indices are guaranteed to be valid by construction and point to a valid operation.
    op_indices: Vec<usize>,
    /// Ensures that `BytecodeMapped` remains consistent for the given `Op` type.
    _op_ty: core::marker::PhantomData<Op>,
}

/// A slice into a [`BytecodeMapped`] instance.
#[derive(Clone, Copy, Debug)]
pub struct BytecodeMappedSlice<'a, Op> {
    /// The full bytecode slice from the original `BytecodeMapped`.
    bytecode: &'a [u8],
    /// Some subslice into the `op_indices` of the original `BytecodeMapped`.
    op_indices: &'a [usize],
    /// Ensures that `BytecodeMapped` remains consistent for the given `Op` type.
    _op_ty: core::marker::PhantomData<Op>,
}

impl<Op> BytecodeMapped<Op> {
    /// Push a single operation onto the bytecode mapping.
    pub fn push_op(&mut self, op: Op)
    where
        Op: ToBytes,
    {
        self.op_indices.push(self.bytecode.len());
        self.bytecode.extend(op.to_bytes());
    }

    /// The inner slice of bytecode that has been mapped.
    pub fn bytecode(&self) -> &[u8] {
        &self.bytecode
    }

    /// The slice of operation indices within the mapped bytecode.
    pub fn op_indices(&self) -> &[usize] {
        &self.op_indices
    }

    /// Slice the op indices from the given index.
    ///
    /// The returned slice represents the remainder of the program from the given op.
    ///
    /// Returns `None` if `start` is out of range of the `op_indices` slice.
    pub fn ops_from(&self, start: usize) -> Option<BytecodeMappedSlice<Op>> {
        Some(BytecodeMappedSlice {
            bytecode: &self.bytecode,
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
        expect_ops_from_indices(&self.bytecode, self.op_indices.iter().copied())
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
        let mut op_indices = Vec::with_capacity(bytecode.len() / std::mem::size_of::<Op>());
        let mut iter_enum = bytecode.iter().enumerate();
        while let Some((ix, &opcode_byte)) = iter_enum.next() {
            let opcode = Op::Opcode::try_from(opcode_byte)?;
            let mut op_bytes = iter_enum.by_ref().map(|(_, &byte)| byte);
            let _op = opcode.parse_op(&mut op_bytes)?;
            op_indices.push(ix);
        }
        Ok(BytecodeMapped {
            bytecode,
            op_indices,
            _op_ty: core::marker::PhantomData,
        })
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
