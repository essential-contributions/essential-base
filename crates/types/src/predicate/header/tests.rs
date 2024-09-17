use super::*;

#[allow(dead_code)]
const fn overhead(num_state_reads: usize, num_constraints: usize) -> usize {
    EncodedFixedSizeHeader::SIZE
        + num_state_reads * core::mem::size_of::<u16>()
        + num_constraints * core::mem::size_of::<u16>()
}

#[allow(clippy::assertions_on_constants)]
const _CHECK_SIZES: () = {
    const ALL_STATE_READS: usize =
        overhead(Predicate::MAX_STATE_READS, 0) + Predicate::MAX_STATE_READS;
    assert!(ALL_STATE_READS < Predicate::MAX_BYTES);

    const LARGE_STATE_READ: usize = overhead(1, 0) + Predicate::MAX_STATE_READ_SIZE_BYTES;
    assert!(LARGE_STATE_READ < Predicate::MAX_BYTES);

    const ALL_CONSTRAINTS: usize =
        overhead(Predicate::MAX_CONSTRAINTS, 0) + Predicate::MAX_CONSTRAINTS;
    assert!(ALL_CONSTRAINTS < Predicate::MAX_BYTES);

    const LARGE_CONSTRAINT: usize = overhead(1, 0) + Predicate::MAX_CONSTRAINT_SIZE_BYTES;
    assert!(LARGE_CONSTRAINT < Predicate::MAX_BYTES);

    // Ensure sizes fit in types.
    assert!(Predicate::MAX_STATE_READ_SIZE_BYTES <= u16::MAX as usize);
    assert!(Predicate::MAX_CONSTRAINT_SIZE_BYTES <= u16::MAX as usize);

    assert!(Predicate::MAX_STATE_READS <= u8::MAX as usize);
    assert!(Predicate::MAX_CONSTRAINTS <= u8::MAX as usize);
};

#[test]
fn test_encoded_size() {
    let mut size = EncodedSize {
        num_state_reads: 10,
        ..Default::default()
    };

    // Header + 10 lengths at 2 bytes each.
    let mut expect = EncodedFixedSizeHeader::SIZE + 10 * core::mem::size_of::<u16>();
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
}

#[test]
fn test_encode_program_lengths() {
    let predicate = Predicate {
        state_read: (0..3).map(|i| vec![0_u8; i]).collect(),
        constraints: (255..259).map(|i| vec![0_u8; i]).collect(),
    };

    let lens = encode_program_lengths(&predicate);
    let expected = vec![0, 0, 0, 1, 0, 2, 0, 255, 1, 0, 1, 1, 1, 2];
    assert_eq!(lens, expected);
}

#[test]
fn test_check_predicate_bounds() {
    let mut bounds = PredicateBounds {
        num_state_reads: Predicate::MAX_STATE_READS,
        num_constraints: Predicate::MAX_CONSTRAINTS,
        state_read_lens: vec![
            Predicate::MAX_STATE_READ_SIZE_BYTES / 2 - 1,
            Predicate::MAX_STATE_READ_SIZE_BYTES / 2 - 1,
        ]
        .into_iter(),
        constraint_lens: vec![
            Predicate::MAX_CONSTRAINT_SIZE_BYTES / 2 - 1,
            Predicate::MAX_CONSTRAINT_SIZE_BYTES / 2 - 1,
        ]
        .into_iter(),
    };

    check_predicate_bounds(bounds.clone()).unwrap();

    bounds.num_state_reads = Predicate::MAX_STATE_READS + 1;

    let err = check_predicate_bounds(bounds.clone()).unwrap_err();
    assert!(matches!(err, PredicateError::TooManyStateReads(_)));

    bounds.num_state_reads = 0;
    bounds.num_constraints = Predicate::MAX_CONSTRAINTS + 1;
    let err = check_predicate_bounds(bounds.clone()).unwrap_err();
    assert!(matches!(err, PredicateError::TooManyConstraints(_)));

    bounds.num_constraints = 0;
    bounds.state_read_lens = vec![Predicate::MAX_STATE_READ_SIZE_BYTES; 6].into_iter();
    let err = check_predicate_bounds(bounds.clone()).unwrap_err();
    assert!(matches!(err, PredicateError::PredicateTooLarge(_)));

    bounds.state_read_lens = vec![1; 6].into_iter();
    bounds.constraint_lens = vec![Predicate::MAX_CONSTRAINT_SIZE_BYTES; 6].into_iter();
    let err = check_predicate_bounds(bounds.clone()).unwrap_err();
    assert!(matches!(err, PredicateError::PredicateTooLarge(_)));
}

#[test]
fn test_try_into_fixed_size_header() {
    let mut predicate = Predicate {
        state_read: vec![vec![0; 10]; 10],
        constraints: vec![vec![0; 10]; 10],
    };

    let header: FixedSizeHeader = (&predicate).try_into().unwrap();
    assert_eq!(header.num_state_reads, 10);
    assert_eq!(header.num_constraints, 10);

    predicate.state_read = vec![vec![]; 256];

    let header: Result<FixedSizeHeader, _> = (&predicate).try_into();
    assert!(header.is_err());
}

#[test]
fn test_try_into_encoded_fixed_size_header() {
    let predicate = Predicate {
        state_read: vec![],
        constraints: vec![vec![]; 255],
    };

    let header: EncodedFixedSizeHeader = (&predicate).try_into().unwrap();
    let expected = [0, 255];
    assert_eq!(header.0, expected);
}

#[test]
fn test_try_into_encoded_header() {
    let predicate = Predicate {
        state_read: (12..15).map(|i| vec![0; i]).collect(),
        constraints: (300..302).map(|i| vec![0; i]).collect(),
    };

    let header: EncodedHeader = (&predicate).try_into().unwrap();
    let expected = EncodedHeader {
        fixed_size_header: EncodedFixedSizeHeader([3, 2]),
        lens: vec![0, 12, 0, 13, 0, 14, 1, 44, 1, 45],
    };
    assert_eq!(header, expected);
}

#[test]
fn test_buffer_indices() {
    let buf = [0, 1];
    assert_eq!(&buf[FixedSizeHeader::num_state_reads_ix()], &[0]);
    assert_eq!(&buf[FixedSizeHeader::num_constraints_ix()], &[1]);
}

#[test]
fn test_fixed_size_header_getters() {
    let buf = [0, 1];
    assert_eq!(FixedSizeHeader::get_num_state_reads(&buf), 0u8);
    assert_eq!(FixedSizeHeader::get_num_constraints(&buf), 1u8);
}

#[test]
fn test_program_lens() {
    let buf = [2, 3, 0, 5, 1, 12, 1, 17, 2, 3, 0, 4, 9, 9];
    let header = FixedSizeHeader::decode(&buf);
    assert_eq!(header.get_state_read_lens_bytes(&buf), &[0, 5, 1, 12]);
    assert_eq!(header.get_constraint_lens_bytes(&buf), &[1, 17, 2, 3, 0, 4]);

    assert_eq!(
        header.decode_state_read_lens(&buf).collect::<Vec<_>>(),
        vec![5, 268]
    );

    assert_eq!(
        header.decode_constraint_lens(&buf).collect::<Vec<_>>(),
        vec![273, 515, 4]
    );
}

#[test]
fn test_decode_fixed_size_header() {
    let buf = [12, 99, 44, 44];
    let header = FixedSizeHeader::decode(&buf);
    assert_eq!(header.num_state_reads, 12);
    assert_eq!(header.num_constraints, 99);
}

#[test]
fn test_fixed_size_header_len() {
    FixedSizeHeader::check_len(0).unwrap_err();
    FixedSizeHeader::check_len(1).unwrap_err();
    FixedSizeHeader::check_len(2).unwrap();
    FixedSizeHeader::check_len(20).unwrap();

    let len = FixedSizeHeader {
        num_state_reads: 12,
        num_constraints: 5,
    }
    .header_len_and_program_lens();
    assert_eq!(len, 36);

    let header = FixedSizeHeader {
        num_state_reads: 12,
        num_constraints: 5,
    };
    header.check_header_len_and_program_lens(0).unwrap_err();
    header.check_header_len_and_program_lens(35).unwrap_err();
    header.check_header_len_and_program_lens(36).unwrap();
    header.check_header_len_and_program_lens(300).unwrap();
}

#[test]
fn test_decode_decoded_header() {
    let buf = [];
    DecodedHeader::decode(&buf).unwrap_err();

    let buf = [1, 0];
    DecodedHeader::decode(&buf).unwrap_err();

    let buf = [0, 0, 0, 0, 0];
    let h = DecodedHeader::decode(&buf).unwrap();
    assert_eq!(
        h,
        DecodedHeader {
            state_reads: vec![],
            constraints: vec![],
        }
    );

    let buf = [2, 3, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 99, 99];
    let h = DecodedHeader::decode(&buf).unwrap();
    assert_eq!(
        h,
        DecodedHeader {
            state_reads: vec![12..13, 13..15],
            constraints: vec![15..18, 18..22, 22..27],
        }
    );

    let buf = [1, 0, 255, 255];
    DecodedHeader::decode(&buf).unwrap_err();
}

#[test]
fn test_decode_state_reads() {
    let buf = [2, 0, 0, 2, 0, 3, 55, 66, 77, 88, 99, 22];
    let h = DecodedHeader::decode(&buf).unwrap();
    let state_reads = h.decode_state_read(&buf);
    assert_eq!(state_reads, vec![vec![55, 66], vec![77, 88, 99]]);
}

#[test]
fn test_decode_constraints() {
    let buf = [1, 2, 0, 2, 0, 2, 0, 3, 11, 22, 55, 66, 77, 88, 99, 22];
    let h = DecodedHeader::decode(&buf).unwrap();
    let constraints = h.decode_constraints(&buf);
    assert_eq!(constraints, vec![vec![55, 66], vec![77, 88, 99]]);
}

#[test]
fn test_check_consistency() {
    let original_fh = FixedSizeHeader {
        num_state_reads: 2,
        num_constraints: 3,
    };

    let original_dh = DecodedHeader {
        state_reads: vec![5..6, 6..9],
        constraints: vec![99..103, 88..901, 77..902],
    };
    let fh = original_fh.clone();
    let dh = original_dh.clone();
    dh.check_consistency(&fh).unwrap();

    let mut fh = original_fh.clone();
    fh.num_state_reads = 3;
    dh.check_consistency(&fh).unwrap_err();

    let mut fh = original_fh.clone();
    fh.num_constraints = 1;
    dh.check_consistency(&fh).unwrap_err();

    let mut dh = original_dh.clone();
    dh.state_reads = vec![5..6, 6..9, 9..10];
    dh.check_consistency(&original_fh).unwrap_err();

    let mut dh = original_dh.clone();
    dh.constraints = vec![99..103, 88..901];
    dh.check_consistency(&original_fh).unwrap_err();
}

#[test]
fn test_header_round_trips() {
    let predicate = Predicate {
        state_read: (0..3).map(|i| vec![0_u8; i]).collect(),
        constraints: (255..259).map(|i| vec![0_u8; i]).collect(),
    };

    let encoded = EncodedHeader::try_from(&predicate).unwrap();
    let bytes: Vec<u8> = encoded.into_iter().collect();
    let decoded = DecodedHeader::decode(&bytes).unwrap();

    assert_eq!(decoded.state_reads.len(), predicate.state_read.len());
    assert_eq!(decoded.constraints.len(), predicate.constraints.len());

    assert!(predicate
        .state_read
        .iter()
        .zip(decoded.state_reads.iter())
        .all(|(a, b)| a.len() == b.len()));

    assert!(predicate
        .constraints
        .iter()
        .zip(decoded.constraints.iter())
        .all(|(a, b)| a.len() == b.len()));
}

#[test]
fn test_bytes_len() {
    let predicate = Predicate {
        state_read: (0..3).map(|i| vec![0_u8; i]).collect(),
        constraints: (255..259).map(|i| vec![0_u8; i]).collect(),
    };
    let encoded = EncodedHeader::try_from(&predicate).unwrap();
    let bytes: Vec<u8> = encoded.into_iter().collect();
    let decoded = DecodedHeader::decode(&bytes).unwrap();

    assert_eq!(decoded.bytes_len(), predicate.encoded_size());
}
