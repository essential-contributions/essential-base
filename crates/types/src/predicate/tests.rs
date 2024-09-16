use super::*;

#[test]
fn test_programs() {
    let predicate = Predicate {
        state_read: (0..3).map(|i| vec![0_u8; i]).collect(),
        constraints: (255..259).map(|i| vec![0_u8; i]).collect(),
        directive: Directive::Minimize(vec![0_u8; 256]),
    };
    let programs = predicate.as_programs().collect::<Vec<_>>();
    assert_eq!(programs.len(), 8);

    for (i, program) in programs[0..3].iter().enumerate() {
        assert_eq!(&predicate.state_read[i], program);
    }

    for (i, program) in programs[3..7].iter().enumerate() {
        assert_eq!(&predicate.constraints[i], program);
    }

    assert_eq!(&vec![0; 256], programs[7]);
}

#[test]
fn test_into_programs() {
    let predicate = Predicate {
        state_read: (0..3).map(|i| vec![0_u8; i]).collect(),
        constraints: (255..259).map(|i| vec![0_u8; i]).collect(),
        directive: Directive::Minimize(vec![0_u8; 256]),
    };
    let programs = predicate.clone().into_programs().collect::<Vec<_>>();
    assert_eq!(programs.len(), 8);

    for (i, program) in programs[0..3].iter().enumerate() {
        assert_eq!(&predicate.state_read[i], program);
    }

    for (i, program) in programs[3..7].iter().enumerate() {
        assert_eq!(&predicate.constraints[i], program);
    }

    assert_eq!(vec![0; 256], programs[7]);
}

#[test]
fn test_encode() {
    let predicate = Predicate {
        state_read: (0..3).map(|i| vec![i as u8; i]).collect(),
        constraints: (200..202).map(|i| vec![i as u8; 2]).collect(),
        directive: Directive::Minimize(vec![33_u8; 2]),
    };
    let bytes: Vec<u8> = predicate.encode().unwrap().collect();

    let expected = vec![
        3, 2, 2, 0, 2, // header
        0, 0, 0, 1, 0, 2, 0, 2, 0, 2, // lens
        1, 2, 2, // state reads
        200, 200, 201, 201, // constraints
        33, 33, // directive
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn test_encoded_size() {
    let predicate = Predicate {
        state_read: (0..3).map(|i| vec![i as u8; i]).collect(),
        constraints: (200..202).map(|i| vec![i as u8; 2]).collect(),
        directive: Directive::Minimize(vec![33_u8; 2]),
    };
    let size = predicate.encoded_size();
    let expected = 5 // header
        + 3 * 2 + 2 * 2 // lens
        + 3 // state reads
        + 4 // constraints
        + 2; // directive
    assert_eq!(size, expected);
}

#[test]
fn test_decode() {
    let bytes = vec![
        3, 2, 2, 0, 2, // header
        0, 0, 0, 1, 0, 2, 0, 2, 0, 2, // lens
        1, 2, 2, // state reads
        200, 200, 201, 201, // constraints
        33, 33, // directive
    ];
    let predicate = Predicate::decode(bytes).unwrap();

    let expected = Predicate {
        state_read: (0..3).map(|i| vec![i as u8; i]).collect(),
        constraints: (200..202).map(|i| vec![i as u8; 2]).collect(),
        directive: Directive::Minimize(vec![33_u8; 2]),
    };
    assert_eq!(predicate, expected);
}

#[test]
fn check_predicate_bounds() {
    let mut predicate = Predicate {
        state_read: vec![],
        constraints: vec![],
        directive: Directive::Satisfy,
    };
    predicate.check_predicate_bounds().unwrap();

    predicate.state_read = (0..(Predicate::MAX_STATE_READS + 1))
        .map(|_| vec![])
        .collect();
    predicate.check_predicate_bounds().unwrap_err();

    predicate.state_read = vec![];
    predicate.constraints = (0..(Predicate::MAX_CONSTRAINTS + 1))
        .map(|_| vec![])
        .collect();
    predicate.check_predicate_bounds().unwrap_err();

    predicate.constraints = vec![];
    predicate.directive = Directive::Minimize(vec![0; Predicate::MAX_DIRECTIVE_SIZE_BYTES + 1]);
    predicate.check_predicate_bounds().unwrap_err();

    predicate.directive = Directive::Satisfy;
    predicate.state_read = vec![vec![0; Predicate::MAX_STATE_READ_SIZE_BYTES + 1]];
    predicate.check_predicate_bounds().unwrap_err();

    predicate.state_read.pop();
    predicate.check_predicate_bounds().unwrap();

    predicate.constraints = vec![vec![0; Predicate::MAX_CONSTRAINT_SIZE_BYTES + 1]];
    predicate.check_predicate_bounds().unwrap_err();

    predicate.constraints.pop();
    predicate.check_predicate_bounds().unwrap();

    predicate.state_read = (0..Predicate::MAX_STATE_READS).map(|_| vec![]).collect();
    predicate.constraints = (0..Predicate::MAX_CONSTRAINTS).map(|_| vec![]).collect();
    predicate.directive = Directive::Minimize(vec![0; Predicate::MAX_DIRECTIVE_SIZE_BYTES]);
    predicate.check_predicate_bounds().unwrap();
}

#[test]
fn test_round_trips() {
    let predicate = Predicate {
        state_read: (0..3).map(|i| vec![i as u8; i]).collect(),
        constraints: (200..202).map(|i| vec![i as u8; 2]).collect(),
        directive: Directive::Minimize(vec![33_u8; 2]),
    };
    let bytes: Vec<u8> = predicate.encode().unwrap().collect();
    let decoded = Predicate::decode(bytes).unwrap();
    assert_eq!(predicate, decoded);
}
