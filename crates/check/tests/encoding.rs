use std::sync::Arc;

use essential_check::constraint_vm;
use essential_types::{
    contract::Contract,
    predicate::Predicate,
    solution::{Solution, SolutionData},
    Hash, PredicateAddress,
};
use sha2::Digest;
use util::State;

pub mod util;

#[tokio::test]
async fn test_encoding_sig_and_pub_key() {
    tracing_subscriber::fmt::init();
    let predicate = Predicate {
        state_read: vec![],
        constraints: vec![constraint_vm::asm::to_bytes([
            // Get the secp256k1 public key. It is 5 slots.
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Stack::Push(5).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            // Hash the key.
            constraint_vm::asm::Stack::Push(5 * 8).into(),
            constraint_vm::asm::Crypto::Sha256.into(),
            // Get the secp256k1 signature. It is 9 slots.
            constraint_vm::asm::Stack::Push(1).into(),
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Stack::Push(9).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            // Recover the public key.
            constraint_vm::asm::Crypto::RecoverSecp256k1.into(),
            // Get the secp256k1 public key. It is 5 slots.
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Stack::Push(5).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            // Compare the two public keys.
            constraint_vm::asm::Stack::Push(5).into(),
            constraint_vm::asm::Pred::EqRange.into(),
        ])
        .collect()],
    };

    let contract = Contract::without_salt(vec![predicate]);

    let address = PredicateAddress {
        contract: essential_hash::content_addr(&contract),
        predicate: essential_hash::content_addr(&contract[0]),
    };

    let (sk, pk) = util::random_keypair([0; 32]);
    let encoded_pk = essential_sign::encode::public_key(&pk);
    let encoded_pk_bytes = essential_sign::encode::public_key_as_bytes(&pk);
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    hasher.update(encoded_pk_bytes);
    let hash: Hash = hasher.finalize().into();

    let secp = secp256k1::Secp256k1::new();
    let sig = secp.sign_ecdsa_recoverable(&secp256k1::Message::from_digest(hash), &sk);
    let encoded_sig = essential_sign::encode::signature(&sig);

    let decision_variables = vec![encoded_pk.to_vec(), encoded_sig.to_vec()];

    let solution = Solution {
        data: vec![SolutionData {
            predicate_to_solve: address.clone(),
            decision_variables,
            state_mutations: vec![],
        }],
    };

    let pre_state = State::EMPTY;
    let post_state = pre_state.clone();

    let predicate = Arc::new(contract[0].clone());
    let get_predicate = |addr: &PredicateAddress| {
        assert_eq!(&address, addr);
        predicate.clone()
    };

    // Run the check, and ensure it returns ok.
    essential_check::solution::check_predicates(
        &pre_state,
        &post_state,
        Arc::new(solution),
        get_predicate,
        Default::default(),
    )
    .await
    .unwrap();
}
