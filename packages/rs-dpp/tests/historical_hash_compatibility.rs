#[path = "fixtures/historical_hashes/core_transactions.rs"]
mod core_transactions;
#[path = "fixtures/historical_hashes/vectors.rs"]
mod vectors;

use dpp::dashcore::consensus::{deserialize, serialize};
use dpp::dashcore::ephemerealdata::chain_lock::ChainLock;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::sml::llmq_type::LLMQType;
use dpp::dashcore::{InstantLock, QuorumHash, Transaction};
use dpp::identity::KeyType;
use dpp::native_bls::NativeBlsModule;
use dpp::platform_value::Value;
use dpp::serialization::{
    PlatformDeserializableUntrusted, PlatformMessageSignable, PlatformSerializable, Signable,
};
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::util::hash::hash_double;
use vectors::*;

#[test]
fn should_preserve_historical_core_transaction_bytes_and_txids() {
    for (name, consensus, txid) in core_transactions::TRANSACTIONS {
        let bytes = hex::decode(consensus).unwrap();
        let transaction: Transaction = deserialize(&bytes).unwrap();
        assert_eq!(serialize(&transaction), bytes, "{name}");
        assert_eq!(transaction.txid().to_string(), *txid, "{name}");
    }
}

#[test]
fn should_preserve_historical_lock_request_and_sign_ids() {
    let instant_bytes = hex::decode(INSTANT_LOCK_CONSENSUS).unwrap();
    let chain_bytes = hex::decode(CHAIN_LOCK_CONSENSUS).unwrap();
    let instant: InstantLock = deserialize(&instant_bytes).unwrap();
    let chain: ChainLock = deserialize(&chain_bytes).unwrap();
    let quorum_type = LLMQType::from(100u8);
    let quorum_hash = QuorumHash::from_byte_array([0x55; 32]);
    assert_eq!(serialize(&instant), instant_bytes);
    assert_eq!(serialize(&chain), chain_bytes);
    assert_eq!(
        hex::encode(instant.request_id().unwrap()),
        INSTANT_LOCK_REQUEST_ID
    );
    assert_eq!(
        hex::encode(chain.request_id().unwrap()),
        CHAIN_LOCK_REQUEST_ID
    );
    assert_eq!(
        hex::encode(instant.sign_id(quorum_type, quorum_hash, None).unwrap()),
        INSTANT_LOCK_SIGN_ID
    );
    assert_eq!(
        hex::encode(chain.sign_id(quorum_type, quorum_hash, None).unwrap()),
        CHAIN_LOCK_SIGN_ID
    );
}

#[test]
fn should_preserve_historical_transfer_preimage_hash_and_signatures() {
    let serialized = hex::decode(TRANSFER_SERIALIZED).unwrap();
    let transition = StateTransition::deserialize_from_bytes_untrusted_exact(&serialized).unwrap();
    assert_eq!(transition.serialize_to_bytes().unwrap(), serialized);
    let bytes = transition.signable_bytes().unwrap();
    assert_eq!(hex::encode(&bytes), TRANSFER_SIGNABLE);
    assert_eq!(hex::encode(hash_double(&bytes)), TRANSFER_HASH);
    for (key_type, public, signature) in [
        (
            KeyType::ECDSA_SECP256K1,
            ECDSA_PUBLIC_KEY,
            TRANSFER_ECDSA_SIGNATURE,
        ),
        (KeyType::BLS12_381, BLS_PUBLIC_KEY, TRANSFER_BLS_SIGNATURE),
    ] {
        let public = hex::decode(public).unwrap();
        let signature = hex::decode(signature).unwrap();
        assert_eq!(
            bytes
                .as_slice()
                .sign_by_private_key(&[1; 32], key_type, &NativeBlsModule)
                .unwrap(),
            signature
        );
        assert!(bytes
            .as_slice()
            .verify_signature(key_type, &public, &signature)
            .is_valid());
        let mut tampered = bytes.clone();
        tampered[0] ^= 1;
        assert!(!tampered
            .as_slice()
            .verify_signature(key_type, &public, &signature)
            .is_valid());
    }
}

#[test]
fn should_preserve_historical_dashpay_ciphertexts_in_the_signed_batch() {
    let serialized = hex::decode(DASHPAY_SERIALIZED).unwrap();
    let mut transition =
        StateTransition::deserialize_from_bytes_untrusted_exact(&serialized).unwrap();
    assert_eq!(transition.serialize_to_bytes().unwrap(), serialized);
    let bytes = transition.signable_bytes().unwrap();
    assert_eq!(hex::encode(&bytes), DASHPAY_SIGNABLE);
    assert_eq!(hex::encode(hash_double(&bytes)), DASHPAY_HASH);
    let signature = hex::decode(DASHPAY_SIGNATURE).unwrap();
    let public = hex::decode(ECDSA_PUBLIC_KEY).unwrap();
    assert_eq!(
        bytes
            .as_slice()
            .sign_by_private_key(&[1; 32], KeyType::ECDSA_SECP256K1, &NativeBlsModule)
            .unwrap(),
        signature
    );
    assert!(bytes
        .as_slice()
        .verify_signature(KeyType::ECDSA_SECP256K1, &public, &signature)
        .is_valid());

    let StateTransition::Batch(BatchTransition::V0(batch)) = &mut transition else {
        panic!("expected a historical document batch");
    };
    let DocumentTransition::Create(DocumentCreateTransition::V0(create)) =
        &mut batch.transitions[0]
    else {
        panic!("expected a historical contactRequest create");
    };
    for (name, frozen) in [
        ("encryptedPublicKey", ENCRYPTED_XPUB),
        ("encryptedAccountLabel", ENCRYPTED_LABEL),
    ] {
        assert_eq!(
            create.data.get(name),
            Some(&Value::Bytes(hex::decode(frozen).unwrap()))
        );
    }
    let Value::Bytes(ciphertext) = create.data.get_mut("encryptedPublicKey").unwrap() else {
        panic!("expected binary ciphertext");
    };
    ciphertext[16] ^= 1;
    let tampered = transition.signable_bytes().unwrap();
    assert_ne!(tampered, bytes);
    assert!(!tampered
        .as_slice()
        .verify_signature(KeyType::ECDSA_SECP256K1, &public, &signature)
        .is_valid());
}
