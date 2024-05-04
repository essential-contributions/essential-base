use essential_sign::sign;
use essential_types::{
    intent::{Directive, Intent},
    slots::Slots,
    Signature,
};
use rand::SeedableRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};

fn test_intent() -> Intent {
    Intent {
        slots: Slots {
            decision_variables: 1,
            state: Default::default(),
        },
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

fn random_keypair(seed: [u8; 32]) -> (SecretKey, PublicKey) {
    let mut rng = rand::rngs::SmallRng::from_seed(seed);
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut rng)
}

#[test]
fn sign_intent() {
    let (sk, _pk) = random_keypair([0xcd; 32]);
    let signed = sign(test_intent(), sk);
    let expected_signature_hex = concat!(
        "641965942f822f382fa5cad859c5906f2acaac4a611e7f8c66d6d1aecde79919",
        "75aa2c9ae3b0d170c78c39acc49fecafca3c13e92c32c031af113eabdf973239"
    );
    let hex = hex::encode(signed.signature.0);
    assert_eq!(expected_signature_hex, hex);
}

#[test]
fn recover() {
    let (sk, pk) = random_keypair([0xcd; 32]);
    let data = test_intent();
    let signed = sign(data, sk);
    let recovered_pk = essential_sign::recover(signed).unwrap();
    assert_eq!(pk, recovered_pk);
}

#[test]
fn fail_to_recover() {
    let (sk, _pk) = random_keypair([0xcd; 32]);
    let data = test_intent();
    let signed = sign(data, sk);
    let mut corrupted_signed = signed.clone();
    corrupted_signed.signature.1 = (corrupted_signed.signature.1 + 1) % 4;
    assert!(essential_sign::recover(corrupted_signed).is_err());
}

#[test]
fn verify_signature() {
    let (sk, _pk) = random_keypair([0xcd; 32]);
    let data = test_intent();
    let signed = sign(data, sk);
    let mut signed_corrupted = signed.clone();
    signed_corrupted.signature = Signature([0u8; 64], 0);
    assert!(essential_sign::verify(&signed).is_ok());
    assert!(!essential_sign::verify(&signed_corrupted).is_ok());
}
