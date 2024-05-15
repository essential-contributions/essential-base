use essential_types::ContentAddress;

#[test]
fn content_address() {
    let ca = ContentAddress([
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ]);
    assert_eq!(
        &format!("{ca:x}"),
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    );
    assert_eq!(
        &format!("{ca:X}"),
        "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F"
    );
    assert_eq!(
        &format!("{ca}"),
        "0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    );
}
