//! Historical fixture generator, invoked by scripts/generate_historical_test_vectors.sh.

use dpp::address_funds::PlatformAddress;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::ephemerealdata::chain_lock::ChainLock;
use dpp::dashcore::hash_types::CycleHash;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dpp::dashcore::sml::llmq_type::LLMQType;
use dpp::dashcore::{BlockHash, InstantLock, OutPoint, QuorumHash, Txid};
use dpp::identity::KeyType;
use dpp::native_bls::NativeBlsModule;
use dpp::platform_value::{Identifier, Value};
use dpp::serialization::{PlatformMessageSignable, PlatformSerializable, Signable};
use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dpp::state_transition::batch_transition::document_create_transition::{
    v0::DocumentCreateTransitionV0, DocumentCreateTransition,
};
use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV0};
use dpp::state_transition::StateTransition;
use dpp::util::hash::hash_double;
use dpp::BlsModule;
use std::collections::BTreeMap;

#[test]
#[ignore = "run through scripts/generate_historical_test_vectors.sh on the pinned revision"]
fn generate() {
    let dir = super::output_dir("signing");
    let write = |name: &str, bytes: &[u8]| {
        std::fs::write(dir.join(format!("{name}.hex")), hex::encode(bytes)).unwrap();
    };
    let transition: StateTransition = AddressFundsTransferTransitionV0 {
        inputs: BTreeMap::from([
            (PlatformAddress::P2pkh([0x11; 20]), (1000, 5000)),
            (PlatformAddress::P2sh([0x22; 20]), (251, 7000)),
        ]),
        outputs: BTreeMap::from([(PlatformAddress::P2pkh([0x33; 20]), 11000)]),
        user_fee_increase: 2,
        ..Default::default()
    }
    .into();
    let bytes = transition.signable_bytes().unwrap();
    write("transfer-signable", &bytes);
    write("transfer-hash", &hash_double(&bytes));
    write(
        "transfer-serialized",
        &transition.serialize_to_bytes().unwrap(),
    );
    let secret = [1; 32];
    let ecdsa = bytes
        .as_slice()
        .sign_by_private_key(&secret, KeyType::ECDSA_SECP256K1, &NativeBlsModule)
        .unwrap();
    let bls = bytes
        .as_slice()
        .sign_by_private_key(&secret, KeyType::BLS12_381, &NativeBlsModule)
        .unwrap();
    write("transfer-ecdsa-signature", &ecdsa);
    write("transfer-bls-signature", &bls);
    write(
        "ecdsa-public-key",
        &PublicKey::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&secret).unwrap())
            .serialize(),
    );
    write(
        "bls-public-key",
        &NativeBlsModule.private_key_to_public_key(&secret).unwrap(),
    );

    let encrypted_key = std::fs::read(dir.join("encrypted-xpub.bin")).unwrap();
    let encrypted_label = std::fs::read(dir.join("encrypted-label.bin")).unwrap();
    let create = DocumentCreateTransitionV0 {
        base: DocumentBaseTransitionV0 {
            id: Identifier::from([0x44; 32]),
            identity_contract_nonce: 1,
            document_type_name: "contactRequest".into(),
            data_contract_id: Identifier::from([0x55; 32]),
        }
        .into(),
        entropy: [0x66; 32],
        data: BTreeMap::from([
            ("toUserId".into(), Value::Bytes(vec![0x77; 32])),
            ("senderKeyIndex".into(), Value::U32(0)),
            ("recipientKeyIndex".into(), Value::U32(1)),
            ("accountReference".into(), Value::U32(2)),
            ("encryptedPublicKey".into(), Value::Bytes(encrypted_key)),
            (
                "encryptedAccountLabel".into(),
                Value::Bytes(encrypted_label),
            ),
        ]),
        ..Default::default()
    };
    let batch = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
        owner_id: Identifier::from([0x33; 32]),
        transitions: vec![DocumentTransition::Create(DocumentCreateTransition::V0(
            create,
        ))],
        ..Default::default()
    }));
    let signable = batch.signable_bytes().unwrap();
    write("dashpay-serialized", &batch.serialize_to_bytes().unwrap());
    write("dashpay-signable", &signable);
    write("dashpay-hash", &hash_double(&signable));
    write(
        "dashpay-signature",
        &signable
            .as_slice()
            .sign_by_private_key(&secret, KeyType::ECDSA_SECP256K1, &NativeBlsModule)
            .unwrap(),
    );

    let signature = BLSSignature::from(std::array::from_fn(|i| i as u8));
    let instant = InstantLock {
        version: 1,
        inputs: vec![OutPoint {
            txid: Txid::from_byte_array(std::array::from_fn(|i| i as u8)),
            vout: 1000,
        }],
        txid: Txid::from_byte_array([0x77; 32]),
        cyclehash: CycleHash::from_byte_array([0x88; 32]),
        signature,
    };
    let chain = ChainLock {
        block_height: 1000,
        block_hash: BlockHash::from_byte_array([0x99; 32]),
        signature,
    };
    let quorum_type = LLMQType::from(100u8);
    let quorum_hash = QuorumHash::from_byte_array([0x55; 32]);
    write(
        "instant-lock-consensus",
        &dpp::dashcore::consensus::serialize(&instant),
    );
    write(
        "instant-lock-request-id",
        instant.request_id().unwrap().as_byte_array(),
    );
    write(
        "instant-lock-sign-id",
        instant
            .sign_id(quorum_type, quorum_hash, None)
            .unwrap()
            .as_byte_array(),
    );
    write(
        "chain-lock-consensus",
        &dpp::dashcore::consensus::serialize(&chain),
    );
    write(
        "chain-lock-request-id",
        chain.request_id().unwrap().as_byte_array(),
    );
    write(
        "chain-lock-sign-id",
        chain
            .sign_id(quorum_type, quorum_hash, None)
            .unwrap()
            .as_byte_array(),
    );
}
