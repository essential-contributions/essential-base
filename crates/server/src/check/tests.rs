use super::*;

#[test]
fn test_packing() {
    let packed: u64 = 0x00ff_ffff_ff00_ff00;
    println!("packed: {:064b}", packed);
    let unpacked = unpack_bytes(packed);
    for (i, byte) in unpacked.iter().enumerate() {
        println!("byte {}: {:08b}", i, byte);
    }
    let result = pack_bytes(&unpacked);
    println!("result: {:064b}", result);
    assert_eq!(packed, result);
}
