use essential_sign::contract::sign;
use essential_types::{contract::Contract, predicate::Predicate, Signature};
use rand::SeedableRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};

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
        "c2475e0e639306c6ac24ad5f6ab6cef38ab10568bd98d83997cbefa878a80d2b",
        "45a4282e8c3f9ecb7f6a557fdce9fb3e20b1eb0bbf27f37c8ac37c5e8de33335"
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
fn verify_signature() {
    let (sk, _pk) = random_keypair([0xcd; 32]);
    let contract = Contract::without_salt(vec![test_predicate()]);
    let signed = sign(contract, &sk);
    let mut signed_corrupted = signed.clone();
    signed_corrupted.signature = Signature([0u8; 64], 0);
    assert!(essential_sign::contract::verify(&signed).is_ok());
    assert!(essential_sign::contract::verify(&signed_corrupted).is_err());
}
