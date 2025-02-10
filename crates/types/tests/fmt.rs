use essential_types::{ContentAddress, Signature};
use prop::test_runner::FileFailurePersistence;
use proptest::{prelude::*, test_runner::Config};

proptest! {
    #![proptest_config(Config::with_failure_persistence(FileFailurePersistence::WithSource("regressions")))]

    #[test]
    fn content_address_roundtrip(bytes in prop::array::uniform32(0u8..)) {
        let ca = ContentAddress(bytes);

        // `fmt::LowerHex`
        let lower_hex = format!("{ca:x}");
        // `fmt::UpperHex`
        let upper_hex = format!("{ca:X}");
        // `fmt::Display`
        let display = format!("{ca}");

        let parsed: ContentAddress = display.parse().unwrap();

        prop_assert_eq!(parsed, ca);
        prop_assert_eq!(lower_hex.len(), 64);
        prop_assert_eq!(upper_hex.len(), 64);
        prop_assert_eq!(upper_hex.to_lowercase(), lower_hex);
        }

    #[test]
    fn signature_roundtrip(compact_sig in any::<[u8; 64]>(), id in any::<u8>()) {
        let sig = Signature(compact_sig, id);

        // `fmt::LowerHex`
        let lower_hex = format!("{sig:x}");
        // `fmt::UpperHex`
        let upper_hex = format!("{sig:X}");
        // `fmt::Display`
        let display = format!("{sig}");

        let parsed: Signature = display.parse().unwrap();

        prop_assert_eq!(parsed, sig);
        prop_assert_eq!(lower_hex.len(), 130);
        prop_assert_eq!(upper_hex.len(), 130);
        prop_assert_eq!(upper_hex.to_lowercase(), lower_hex);
    }
}
