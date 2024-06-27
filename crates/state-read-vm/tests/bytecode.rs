use essential_state_read_vm::{
    asm::{self, Op},
    BytecodeMapped,
};

// This ensures that, in the worst case where there is one operation per
// byte (i.e. there are no `Push` operations), the size of `BytecodeMapped`
// is still at least as or more memory efficient than a `Vec<Op>`.
#[test]
fn mapped_is_compact() {
    assert!(
        core::mem::size_of::<(u8, usize)>() <= core::mem::size_of::<Op>(),
        "The size of a byte and its index must be smaller than or equal \
        to a single `Op` for `BytecodeMapped` to be strictly more memory \
        efficient than (or at least as efficient as) a `Vec<Op>`. If this \
        test has failed and `Op` has become smaller than the size of `(u8, \
        usize)`, then this module can be removed and `Vec<Op>` should be \
        the preferred method of storing operations for execution."
    );
}

// Ensure we can collect a `Result<BytecodeMapped, _>` from an iterator of `Result<Op, _>`.
#[test]
fn mapped_from_op_results() {
    let results: &[Result<Op, _>] = &[
        Ok(asm::Stack::Push(6).into()),
        Ok(asm::Stack::Push(7).into()),
        Ok(asm::Alu::Mul.into()),
        Ok(asm::Stack::Push(42).into()),
        Ok(asm::Pred::Eq.into()),
        Ok(asm::TotalControlFlow::Halt.into()),
    ];
    let mapped: Result<BytecodeMapped, ()> = results.iter().cloned().collect();
    mapped.unwrap();
}

#[test]
fn mapped_from_bytes() {
    let ops = &[
        asm::Stack::Push(6).into(),
        asm::Stack::Push(7).into(),
        asm::Alu::Mul.into(),
        asm::Stack::Push(42).into(),
        asm::Pred::Eq.into(),
        asm::TotalControlFlow::Halt.into(),
    ];
    let bytes: Vec<_> = asm::to_bytes(ops.iter().copied()).collect();

    // Ensure ops to bytecode mapped conversions behave the same.
    let mapped_a: BytecodeMapped = asm::from_bytes(bytes.iter().copied())
        .collect::<Result<_, _>>()
        .unwrap();
    // Check mapping of `Vec<u8>`.
    let mapped_b = BytecodeMapped::try_from(bytes.clone()).unwrap();
    assert_eq!(mapped_a, mapped_b);
    // Check mapping of `&[u8]`.
    let mapped_c = BytecodeMapped::try_from(&bytes[..]).unwrap();
    assert_eq!(mapped_a.as_slice(), mapped_c.as_slice());

    // Ensure the roundtrip conversion is correct.
    let ops_a: Vec<_> = mapped_a.ops().collect();
    let ops_b: Vec<_> = mapped_b.ops().collect();
    let ops_c: Vec<_> = mapped_c.ops().collect();
    assert_eq!(ops_a, ops);
    assert_eq!(ops_b, ops);
    assert_eq!(ops_c, ops);
}
