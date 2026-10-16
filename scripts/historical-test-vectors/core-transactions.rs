//! Historical fixture generator, invoked by scripts/generate_historical_test_vectors.sh.

use dashcore::bls_sig_utils::{BLSPublicKey, BLSSignature};
use dashcore::hashes::Hash;
use dashcore::transaction::special_transaction::{
    provider_registration::{ProviderMasternodeType, ProviderRegistrationPayload},
    provider_update_registrar::ProviderUpdateRegistrarPayload,
    provider_update_revocation::ProviderUpdateRevocationPayload,
    provider_update_service::ProviderUpdateServicePayload,
    TransactionPayload,
};
use dashcore::{OutPoint, PlatformNodeId, PubkeyHash, ScriptBuf, Transaction, TxOut, Txid};

#[test]
#[ignore = "run through scripts/generate_historical_test_vectors.sh on the pinned revision"]
fn generate() {
    let output = super::output_dir("core");
    let key = BLSPublicKey::from(std::array::from_fn(|i| i as u8));
    let node = PlatformNodeId::from_byte_array(std::array::from_fn(|i| i as u8 + 1));
    let registration = ProviderRegistrationPayload {
        version: 2,
        masternode_type: ProviderMasternodeType::HighPerformance,
        masternode_mode: 0,
        collateral_outpoint: OutPoint {
            txid: Txid::from_byte_array([1; 32]),
            vout: 3,
        },
        service_address: "127.0.0.1:19999".parse().unwrap(),
        owner_key_hash: PubkeyHash::from_byte_array([2; 20]),
        operator_public_key: key,
        voting_key_hash: PubkeyHash::from_byte_array([3; 20]),
        operator_reward: 100,
        script_payout: ScriptBuf::from(vec![0x51]),
        inputs_hash: dashcore::hash_types::InputsHash::from_byte_array([4; 32]),
        signature: vec![5; 65],
        platform_node_id: Some(node),
        platform_p2p_port: Some(26656),
        platform_http_port: Some(443),
    };
    let mut regular = registration.clone();
    regular.masternode_type = ProviderMasternodeType::Regular;
    regular.platform_node_id = None;
    regular.platform_p2p_port = None;
    regular.platform_http_port = None;
    let mut records = vec![
        (
            "registration-evo",
            TransactionPayload::ProviderRegistrationPayloadType(registration),
        ),
        (
            "registration-regular",
            TransactionPayload::ProviderRegistrationPayloadType(regular),
        ),
        (
            "registrar",
            TransactionPayload::ProviderUpdateRegistrarPayloadType(
                ProviderUpdateRegistrarPayload {
                    version: 2,
                    pro_tx_hash: Txid::from_byte_array([1; 32]),
                    provider_mode: 0,
                    operator_public_key: key,
                    voting_key_hash: PubkeyHash::from_byte_array([3; 20]),
                    script_payout: ScriptBuf::from(vec![0x51]),
                    inputs_hash: dashcore::hash_types::InputsHash::from_byte_array([4; 32]),
                    payload_sig: vec![5; 65],
                },
            ),
        ),
        (
            "service-evo",
            TransactionPayload::ProviderUpdateServicePayloadType(ProviderUpdateServicePayload {
                version: 2,
                mn_type: Some(1),
                pro_tx_hash: Txid::from_byte_array([1; 32]),
                ip_address: 1,
                port: 19999,
                script_payout: ScriptBuf::from(vec![0x51]),
                inputs_hash: dashcore::hash_types::InputsHash::from_byte_array([4; 32]),
                platform_node_id: Some(node),
                platform_p2p_port: Some(26656),
                platform_http_port: Some(443),
                payload_sig: BLSSignature::from([6; 96]),
            }),
        ),
        (
            "revocation",
            TransactionPayload::ProviderUpdateRevocationPayloadType(
                ProviderUpdateRevocationPayload {
                    version: 2,
                    pro_tx_hash: Txid::from_byte_array([1; 32]),
                    reason: 1,
                    inputs_hash: dashcore::hash_types::InputsHash::from_byte_array([4; 32]),
                    payload_sig: BLSSignature::from([6; 96]),
                },
            ),
        ),
    ];
    for (name, version) in [("coinbase-v1", 1), ("coinbase-v2", 2), ("coinbase-v3", 3)] {
        records.push((
            name,
            TransactionPayload::CoinbasePayloadType(
                dashcore::transaction::special_transaction::coinbase::CoinbasePayload {
                    version,
                    height: 123,
                    merkle_root_masternode_list:
                        dashcore::hash_types::MerkleRootMasternodeList::from_byte_array([8; 32]),
                    merkle_root_quorums: dashcore::hash_types::MerkleRootQuorums::from_byte_array(
                        [9; 32],
                    ),
                    best_cl_height: (version >= 3).then_some(100),
                    best_cl_signature: (version >= 3).then_some(BLSSignature::from([6; 96])),
                    asset_locked_amount: (version >= 3).then_some(5000),
                },
            ),
        ));
    }
    for (name, payload) in records {
        let transaction = Transaction {
            version: 3,
            lock_time: 42,
            input: vec![],
            output: vec![TxOut {
                value: 1234,
                script_pubkey: ScriptBuf::from(vec![0x51]),
            }],
            special_transaction_payload: Some(payload),
        };
        std::fs::write(
            output.join(format!("{name}.consensus")),
            dashcore::consensus::serialize(&transaction),
        )
        .unwrap();
        std::fs::write(
            output.join(format!("{name}.txid")),
            transaction.txid().to_string(),
        )
        .unwrap();
    }
}
