use super::*;

#[test]
fn test_logical_shifts() {
    let i = 0xFF00;
    let r = shl(i, 8).unwrap();
    assert_eq!(r, 0xFF0000);
    let r = shr(r, 16).unwrap();
    assert_eq!(r, 0xFF);
    let r = shl(r, 60).unwrap();
    let r = shl(r, 60).unwrap();
    assert_eq!(r, 0x0);

    let r = shr(i, 24).unwrap();
    assert_eq!(r, 0x0);
}

#[test]
fn test_arithmetic_shr() {
    let i: Word = 100;
    let r = arithmetic_shr(i, 1).unwrap();
    assert_eq!(r, i / 2);
    let r = arithmetic_shr(i, 2).unwrap();
    assert_eq!(r, i / 4);
    let r = arithmetic_shr(i, 3).unwrap();
    assert_eq!(r, i / 8);

    let i: Word = -120;
    let r = arithmetic_shr(i, 1).unwrap();
    assert_eq!(r, i / 2);
    let r = arithmetic_shr(i, 2).unwrap();
    assert_eq!(r, i / 4);
    let r = arithmetic_shr(i, 3).unwrap();
    assert_eq!(r, i / 8);
}

#[test]
fn test_shift_bounds() {
    check_shift_bounds(0).unwrap();
    check_shift_bounds(1).unwrap();
    check_shift_bounds(BITS_IN_WORD - 1).unwrap();
    check_shift_bounds(BITS_IN_WORD).unwrap_err();
    check_shift_bounds(BITS_IN_WORD + 1).unwrap_err();
}
