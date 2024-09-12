use crate::{contract::Contract, Word};

use super::*;

#[allow(dead_code)]
const fn overhead(num_state_reads: usize, num_constraints: usize) -> usize {
    StaticHeader::SIZE
        + num_state_reads * core::mem::size_of::<u16>()
        + num_constraints * core::mem::size_of::<u16>()
}

#[allow(clippy::assertions_on_constants)]
const _CHECK_SIZES: () = {
    const ALL_STATE_READS: usize =
        overhead(Predicate::MAX_STATE_READS, 0) + Predicate::MAX_STATE_READS;
    assert!(ALL_STATE_READS < Predicate::MAX_PREDICATE_BYTES);

    const LARGE_STATE_READ: usize = overhead(1, 0) + Predicate::MAX_STATE_READ_SIZE_BYTES;
    assert!(LARGE_STATE_READ < Predicate::MAX_PREDICATE_BYTES);

    const ALL_CONSTRAINTS: usize =
        overhead(Predicate::MAX_CONSTRAINTS, 0) + Predicate::MAX_CONSTRAINTS;
    assert!(ALL_CONSTRAINTS < Predicate::MAX_PREDICATE_BYTES);

    const LARGE_CONSTRAINT: usize = overhead(1, 0) + Predicate::MAX_CONSTRAINT_SIZE_BYTES;
    assert!(LARGE_CONSTRAINT < Predicate::MAX_PREDICATE_BYTES);

    assert!(Predicate::MAX_DIRECTIVE_SIZE_BYTES < Predicate::MAX_PREDICATE_BYTES);

    // Ensure sizes fit in types.
    assert!(Predicate::MAX_DIRECTIVE_SIZE_BYTES <= u16::MAX as usize);
    assert!(Predicate::MAX_STATE_READ_SIZE_BYTES <= u16::MAX as usize);
    assert!(Predicate::MAX_CONSTRAINT_SIZE_BYTES <= u16::MAX as usize);

    assert!(Predicate::MAX_STATE_READS <= u8::MAX as usize);
    assert!(Predicate::MAX_CONSTRAINTS <= u8::MAX as usize);
};

#[test]
fn test_directive() {
    let mut predicate = Predicate {
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    };

    let r: StaticHeaderLayout = (&predicate).try_into().unwrap();
    assert_eq!(r.directive_len, 0);

    predicate.directive = Directive::Maximize(vec![0; 32]);
    let r: StaticHeaderLayout = (&predicate).try_into().unwrap();
    assert_eq!(r.directive_len, 32);

    predicate.directive = Directive::Minimize(vec![0; 20]);
    let r: StaticHeaderLayout = (&predicate).try_into().unwrap();
    assert_eq!(r.directive_len, 20);
}

#[test]
fn test_encoded_size() {
    let mut size = EncodedSize {
        num_state_reads: 10,
        ..Default::default()
    };

    // Header + 10 lengths at 2 bytes each.
    let mut expect = StaticHeader::SIZE + 10 * core::mem::size_of::<u16>();
    assert_eq!(encoded_size(&size), expect);

    // Add 20 constraint lens.
    size.num_constraints = 20;
    expect += 20 * core::mem::size_of::<u16>();
    assert_eq!(encoded_size(&size), expect);

    // Add 1500 bytes of state reads.
    size.state_read_lens_sum = 1500;
    expect += 1500;
    assert_eq!(encoded_size(&size), expect);

    // Add 2000 bytes of constraints.
    size.constraint_lens_sum = 2000;
    expect += 2000;
    assert_eq!(encoded_size(&size), expect);

    // Add 100 bytes of directive.
    size.directive_size = 100;
    expect += 100;
    assert_eq!(encoded_size(&size), expect);
}
