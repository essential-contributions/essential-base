use std::sync::Arc;

use essential_check::constraint_vm;
use essential_types::{
    intent::{Directive, Intent},
    solution::{Solution, SolutionData},
    Hash, IntentAddress,
};
use sha2::Digest;
use util::State;

pub mod util;

#[tokio::test]
async fn test_encoding_sig_and_pub_key() {
    tracing_subscriber::fmt::init();
    let intent = Intent {
        state_read: vec![],
        constraints: vec![constraint_vm::asm::to_bytes([
            // Get the secp256k1 public key. It is 5 slots.
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(1).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(2).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(3).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(4).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            // Hash the key.
            constraint_vm::asm::Stack::Push(5).into(),
            constraint_vm::asm::Crypto::Sha256.into(),
            // Get the secp256k1 signature. It is 8 slots.
            constraint_vm::asm::Stack::Push(5).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(6).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(7).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(8).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(9).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(10).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(11).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(12).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(13).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            // Recover the public key.
            constraint_vm::asm::Crypto::RecoverSecp256k1.into(),
            // Get the secp256k1 public key. It is 5 slots.
            constraint_vm::asm::Stack::Push(0).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(1).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(2).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(3).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            constraint_vm::asm::Stack::Push(4).into(),
            constraint_vm::asm::Access::DecisionVar.into(),
            // Compare the two public keys.
            constraint_vm::asm::Stack::Push(5).into(),
            constraint_vm::asm::Pred::EqRange.into(),
        ])
        .collect()],
        directive: Directive::Satisfy,
    };

    let set = vec![intent];

    let address = IntentAddress {
        set: essential_hash::intent_set_addr::from_intents(&set),
        intent: essential_hash::content_addr(&set[0]),
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

    let mut decision_variables = vec![];
    decision_variables.extend(encoded_pk.iter().map(|&i| vec![i]));
    decision_variables.extend(encoded_sig.iter().map(|&i| vec![i]));

    let solution = Solution {
        data: vec![SolutionData {
            intent_to_solve: address.clone(),
            decision_variables,
            transient_data: vec![],
            state_mutations: vec![],
        }],
    };

    let pre_state = State::EMPTY;
    let post_state = pre_state.clone();

    let intent = Arc::new(set[0].clone());
    let get_intent = |addr: &IntentAddress| {
        assert_eq!(&address, addr);
        intent.clone()
    };

    // Run the check, and ensure util is 1.
    let (util, _) = essential_check::solution::check_intents(
        &pre_state,
        &post_state,
        Arc::new(solution),
        get_intent,
        Default::default(),
    )
    .await
    .unwrap();

    // Util should be 1 - only one solved intent.
    assert_eq!(util, 1.0);
}
