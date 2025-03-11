use essential_hash::hash_bytes;
use essential_sign::contract::sign;
use essential_types::{contract::Contract, predicate::Predicate};
use rand::SeedableRng;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

use essential_sign::sign_message;

fn test_predicate() -> Predicate {
    Predicate::default()
}

fn random_keypair(seed: [u8; 32]) -> (SecretKey, PublicKey) {
    let mut rng = rand::rngs::SmallRng::from_seed(seed);
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut rng)
}

#[test]
fn sign_predicate() {
    let (sk, _pk) = random_keypair([0xcd; 32]);
    let contract = Contract::without_salt(vec![test_predicate()]);
    let signed = sign(contract, &sk);
    let expected_signature_hex = concat!(
        "0b01fc0dffd2fd66b21c1a1d8d426834a131f30faf617a9c5862c0a09c7c217f",
        "46d155dc91c014918d6c082eca518d9535fd44d5aa849f80c09589a36818f867",
    );
    let hex = hex::encode(signed.signature.0);
    assert_eq!(expected_signature_hex, hex);
}

#[test]
fn recover() {
    let (sk, pk) = random_keypair([0xcd; 32]);
    let contract = Contract::without_salt(vec![test_predicate()]);
    let signed = sign(contract, &sk);
    let recovered_pk = essential_sign::contract::recover(&signed).unwrap();
    assert_eq!(pk, recovered_pk);
}

#[test]
fn fail_to_recover() {
    let (sk, _pk) = random_keypair([0xcd; 32]);
    let contract = Contract::without_salt(vec![test_predicate()]);
    let signed = sign(contract, &sk);
    let mut corrupted_signed = signed.clone();
    corrupted_signed.signature.1 = (corrupted_signed.signature.1 + 2) % 4;
    assert!(essential_sign::contract::recover(&corrupted_signed).is_err());
}

#[test]
fn verify_pubkey() {
    let (sk, pk) = random_keypair([0xcd; 32]);
    let (_sk2, pk2) = random_keypair([0xab; 32]);

    let data = b"Essential";
    let hash = hash_bytes(data);
    let msg = Message::from_digest(hash);
    let signed_message = sign_message(&msg, &sk);

    assert!(essential_sign::verify_message(&msg, &signed_message.0, &pk).is_ok());
    assert!(essential_sign::verify_message(&msg, &signed_message.0, &pk2).is_err());
}
