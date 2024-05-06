use essential_types::convert::u8_32_from_word_4;

use crate::{
    asm::{Crypto, Op, Stack, Word},
    crypto::{bytes_from_words, recover_secp256k1},
    error::{ConstraintError, CryptoError, OpError},
    eval_ops, exec_ops,
    test_util::*,
    types::{
        convert::{bytes_from_word, word_4_from_u8_32, word_8_from_u8_64},
        Hash,
    },
};

fn exec_ops_sha256(ops: &[Op]) -> Hash {
    let stack = exec_ops(ops, TEST_ACCESS).unwrap();
    assert_eq!(stack.len(), 4);
    let bytes: Vec<u8> = stack.iter().copied().flat_map(bytes_from_word).collect();
    bytes.try_into().unwrap()
}

#[test]
#[rustfmt::skip]
fn sha256_1_word() {
    let ops = &[
        Stack::Push(0x0000000000000000).into(), // Data
        Stack::Push(1).into(), // Data Length
        Crypto::Sha256.into(),
    ];
    let hash = exec_ops_sha256(ops);
    // Value retrieved externally using command:
    // $ echo "0000000000000000" | xxd -r -p | sha256sum
    let expected = [
        0xaf, 0x55, 0x70, 0xf5, 0xa1, 0x81, 0x0b, 0x7a,
        0xf7, 0x8c, 0xaf, 0x4b, 0xc7, 0x0a, 0x66, 0x0f,
        0x0d, 0xf5, 0x1e, 0x42, 0xba, 0xf9, 0x1d, 0x4d,
        0xe5, 0xb2, 0x32, 0x8d, 0xe0, 0xe8, 0x3d, 0xfc,
    ];
    assert_eq!(&hash[..], &expected);
}

#[test]
#[rustfmt::skip]
fn sha256_3_words() {
    let ops = &[
        Stack::Push(0x00000000000000FF).into(), // Data
        Stack::Push(0x00000000000000FF).into(), // Data
        Stack::Push(0x00000000000000FF).into(), // Data
        Stack::Push(3).into(), // Data Length
        Crypto::Sha256.into(),
    ];
    let hash = exec_ops_sha256(ops);
    // Value retrieved externally using command:
    // $ echo "00000000000000FF00000000000000FF00000000000000FF" | xxd -r -p | sha256sum
    let expected = [
        0x58, 0x2d, 0xc8, 0xbd, 0xf8, 0xed, 0x36, 0x46,
        0x65, 0xa2, 0xd4, 0x59, 0x13, 0xc4, 0x79, 0x9f,
        0x38, 0x6e, 0xe0, 0xc2, 0x51, 0x96, 0x80, 0x81,
        0x00, 0xe2, 0xfc, 0x2d, 0xae, 0x75, 0x00, 0xd6,
    ];
    assert_eq!(&hash[..], &expected);
}

// Generate some test operations for a successful ed25519 verification.
fn test_ed25519_ops() -> Vec<Op> {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::{Rng, SeedableRng};

    // Test data.
    let data: &[Word] = &[7, 3, 5, 7];
    let data_bytes: Vec<_> = bytes_from_words(data.iter().copied()).collect();

    // Generate keys.
    let mut rng = rand::rngs::SmallRng::from_seed([0x00; 32]);
    let key_bytes = rng.gen();
    let privkey: SigningKey = SigningKey::from_bytes(&key_bytes);
    let pubkey = privkey.verifying_key();
    let pubkey_bytes = pubkey.to_bytes();

    // Sign the data and check it verifies.
    let signature = privkey.sign(&data_bytes);
    pubkey.verify_strict(&data_bytes, &signature).unwrap();
    let signature_bytes = signature.to_bytes();

    // Push the data, length, signature, pubkey and finally the `VerifyEd25519` op.
    data.iter()
        .copied()
        .chain(Some(Word::try_from(data.len()).unwrap()))
        .chain(word_8_from_u8_64(signature_bytes))
        .chain(word_4_from_u8_32(pubkey_bytes))
        .map(Stack::Push)
        .map(Op::from)
        .chain(Some(Crypto::VerifyEd25519.into()))
        .collect()
}

#[test]
fn verify_ed25519_true() {
    let ops = test_ed25519_ops();
    assert!(eval_ops(&ops, TEST_ACCESS).unwrap());
}

#[test]
fn verify_ed25519_false() {
    let mut ops = test_ed25519_ops();
    ops[0] = Stack::Push(0).into(); // Invalidate data.
    assert!(!eval_ops(&ops, TEST_ACCESS).unwrap());
}

#[test]
fn ed25519_error() {
    let mut ops = test_ed25519_ops();
    // Invalidate pubkey.
    let key_ix = ops.len() - 5;
    ops[key_ix] = Stack::Push(1).into();
    ops[key_ix + 1] = Stack::Push(1).into();
    ops[key_ix + 2] = Stack::Push(1).into();
    ops[key_ix + 3] = Stack::Push(1).into();
    let res = eval_ops(&ops, TEST_ACCESS);
    match res {
        Err(ConstraintError::Op(_, OpError::Crypto(CryptoError::Ed25519(_err)))) => (),
        _ => panic!("expected ed25519 error, got {res:?}"),
    }
}

#[test]
fn test_secp256k1() {
    use rand::SeedableRng;
    use secp256k1::{Message, PublicKey, Secp256k1};

    let secp = Secp256k1::new();
    let mut rng = rand::rngs::SmallRng::from_seed([0x00; 32]);
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);

    let message = Message::from_digest([0; 32]);

    let secp = Secp256k1::new();
    let sig = secp.sign_ecdsa_recoverable(&message, &secret_key);
    let (rec_id, sig_bytes) = sig.serialize_compact();

    let check = |sig_bytes, message: [u8; 32]| {
        let mut stack = crate::Stack::default();
        stack.extend(word_4_from_u8_32(message)).unwrap();
        stack.extend(word_8_from_u8_64(sig_bytes)).unwrap();
        let rec_id_word = Word::from(rec_id.to_i32());
        stack.push(rec_id_word).unwrap();

        recover_secp256k1(&mut stack).unwrap();

        let end = stack.pop().unwrap();
        let end = bytes_from_word(end);
        let recovered = stack.pop4().unwrap();
        let mut recovered = u8_32_from_word_4(recovered).to_vec();
        recovered.push(end[7]);
        let bytes: [u8; 33] = recovered.try_into().unwrap();
        bytes
    };

    let result = PublicKey::from_slice(&check(sig_bytes, *message.as_ref())).unwrap();

    // Recovered successfully
    assert_eq!(result, public_key);

    let result = PublicKey::from_slice(&check(sig_bytes, [1; 32])).unwrap();

    // Recovered wrong public key
    assert_ne!(result, public_key);

    let result = check([0; 64], *message.as_ref());

    // Invalid signature
    assert_eq!(result, [0u8; 33]);
}
