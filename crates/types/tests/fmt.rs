use essential_types::{ContentAddress, Signature};

#[test]
fn content_address() {
    let ca = ContentAddress([
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ]);

    // `fmt::LowerHex`
    assert_eq!(
        &format!("{ca:x}"),
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    );

    // `fmt::UpperHex`
    assert_eq!(
        &format!("{ca:X}"),
        "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F"
    );

    // `fmt::Display`
    let ca_string = format!("{ca}");
    assert_eq!(
        &ca_string,
        "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F"
    );

    // `str::FromStr`
    let ca2: ContentAddress = ca_string.parse().unwrap();
    assert_eq!(ca, ca2);
}

#[test]
fn signature() {
    let sig = Signature(
        [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ],
        3,
    );

    // `fmt::LowerHex`
    assert_eq!(
        &format!("{sig:x}"),
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f03"
    );

    // `fmt::UpperHex`
    assert_eq!(
        &format!("{sig:X}"),
        "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F202122232425262728292A2B2C2D2E2F303132333435363738393A3B3C3D3E3F03"
    );

    // `fmt::Display`
    let sig_string = format!("{sig}");
    assert_eq!(
        &sig_string,
        "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F202122232425262728292A2B2C2D2E2F303132333435363738393A3B3C3D3E3F03"
    );

    // `str::FromStr`
    let sig2: Signature = sig_string.parse().unwrap();
    assert_eq!(sig, sig2);
}
