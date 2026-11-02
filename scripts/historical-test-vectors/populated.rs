//! Historical fixture generator, invoked by scripts/generate_historical_test_vectors.sh.

use dpp::core_types::validator::v0::ValidatorV0;
use dpp::core_types::validator_set::{v0::ValidatorSetV0, ValidatorSet};
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::dashcore::{ProTxHash, PubkeyHash, Txid};
use dpp::dashcore_rpc::json::QuorumType;
use dpp::dashcore_rpc::json::{MasternodeListItem, MasternodeType};
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use drive_abci::config::{ChainLockConfig, PlatformConfig};
use drive_abci::platform_types::masternode::{
    v0::{MasternodeStateV0, MasternodeV0},
    Masternode,
};
use drive_abci::platform_types::platform_state::platform_state_for_saving::v2::{
    serialize_masternode_entry, serialize_validator_set_entry,
};
use drive_abci::platform_types::platform_state::{
    platform_state_for_saving::{v2::PlatformStateForSavingV2, PlatformStateForSaving},
    PlatformState,
};
use drive_abci::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSetForSaving, SignatureVerificationQuorumSetV0,
    SignatureVerificationQuorumSetV0Methods, VerificationQuorum,
};
use std::collections::BTreeMap;
use std::path::Path;

const PUBLIC_KEYS: [&str; 4] = [
    "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
    "a572cbea904d67468808c8eb50a9450c9721db309128012543902d0ac358a62ae28f75bb8f1c7c42c39a8c5529bf0f4e",
    "95fde78acd5f6886ddaf5d0056610167c513d09c1c0efabbc7cdcc69beea113779c4a81e2d24daafc5387dbf6ac5fe48",
    "b7f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
];

fn quorums(entries: &[(u8, usize, Option<u32>)]) -> Quorums<VerificationQuorum> {
    entries
        .iter()
        .map(|&(hash, key, index)| {
            (
                QuorumHash::from_byte_array([hash; 32]),
                VerificationQuorum {
                    public_key: hex::decode(PUBLIC_KEYS[key])
                        .unwrap()
                        .as_slice()
                        .try_into()
                        .unwrap(),
                    index,
                },
            )
        })
        .collect()
}

#[test]
#[ignore = "run through scripts/generate_historical_test_vectors.sh on the pinned revision"]
fn generate() {
    let mut runtime = SignatureVerificationQuorumSetV0::new(&ChainLockConfig {
        quorum_type: QuorumType::Llmq400_60,
        quorum_size: 400,
        quorum_window: 288,
        quorum_active_signers: 4,
        quorum_rotation: false,
    });
    runtime.set_current_quorums(quorums(&[(0x11, 0, None), (0x22, 1, Some(0))]));
    runtime.set_previous_past_quorums(Quorums::default(), 899, 900);
    runtime.set_previous_past_quorums(quorums(&[(0x33, 2, Some(1)), (0x44, 3, None)]), 1000, 1008);
    let output_dir = super::output_dir("populated");
    let output = output_dir.as_path();
    let config = bincode::config::standard()
        .with_big_endian()
        .with_no_limit();
    for (name, stored) in [
        (
            "quorum-storage-v1.hex",
            SignatureVerificationQuorumSetForSaving::V1(runtime.clone().into()),
        ),
        (
            "quorum-storage-v2.hex",
            SignatureVerificationQuorumSetForSaving::V2(runtime.clone().into()),
        ),
    ] {
        write(
            output,
            name,
            bincode::encode_to_vec(stored, config).unwrap(),
        );
    }
    let mut state =
        PlatformState::default_with_protocol_versions(13, 14, &PlatformConfig::default()).unwrap();
    state.chain_lock_validating_quorums = runtime.clone().into();
    state.instant_lock_validating_quorums = runtime.into();
    for (tag, node_type, service) in [
        (0x31, MasternodeType::Regular, "1.2.3.4:19999"),
        (0x41, MasternodeType::Evo, "[2001:db8::1]:19999"),
    ] {
        let evo = node_type == MasternodeType::Evo;
        let node: MasternodeListItem = Masternode::V0(MasternodeV0 {
            node_type,
            pro_tx_hash: ProTxHash::from_byte_array([tag; 32]),
            collateral_hash: Txid::from_byte_array([tag + 1; 32]),
            collateral_index: 1000,
            collateral_address: [0x21; 20],
            operator_reward: 1.5,
            state: MasternodeStateV0 {
                service: service.parse().unwrap(),
                registered_height: 100,
                pose_revived_height: Some(200),
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: [0x22; 20],
                voting_address: [0x23; 20],
                payout_address: [0x24; 20],
                pub_key_operator: hex::decode(PUBLIC_KEYS[0]).unwrap(),
                operator_payout_address: Some([0x25; 20]),
                platform_node_id: evo.then_some(std::array::from_fn(|i| i as u8 + 1)),
                platform_p2p_port: evo.then_some(26656),
                platform_http_port: evo.then_some(443),
            },
        })
        .into();
        write(
            output,
            if evo {
                "masternode-evo.hex"
            } else {
                "masternode-regular.hex"
            },
            serialize_masternode_entry(&node, PlatformVersion::latest()).unwrap(),
        );
        if evo {
            state
                .hpmn_masternode_list
                .insert(node.pro_tx_hash, node.clone());
        }
        state.full_masternode_list.insert(node.pro_tx_hash, node);
    }
    let validator = ValidatorV0 {
        pro_tx_hash: ProTxHash::from_byte_array([0x41; 32]),
        public_key: Some(
            hex::decode(PUBLIC_KEYS[1])
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap(),
        ),
        node_ip: "2001:db8::1".into(),
        node_id: PubkeyHash::from_byte_array(std::array::from_fn(|i| i as u8 + 1)),
        core_port: 19999,
        platform_http_port: 443,
        platform_p2p_port: 26656,
        is_banned: false,
    };
    let quorum_hash = QuorumHash::from_byte_array([0x55; 32]);
    let set = ValidatorSet::V0(ValidatorSetV0 {
        quorum_hash,
        quorum_index: Some(2),
        core_height: 1000,
        members: BTreeMap::from([(validator.pro_tx_hash, validator)]),
        threshold_public_key: hex::decode(PUBLIC_KEYS[2])
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap(),
    });
    write(
        output,
        "validator-set-entry.hex",
        serialize_validator_set_entry(&set).unwrap(),
    );
    state.validator_sets.insert(quorum_hash, set);
    state.current_validator_set_quorum_hash = quorum_hash;
    write(
        output,
        "populated-platform-state-v1.hex",
        state.serialize_to_bytes().unwrap(),
    );
    write(
        output,
        "populated-checkpoint-platform-state.hex",
        state.serialize_standalone_to_bytes().unwrap(),
    );
    let stored = PlatformStateForSaving::V2(PlatformStateForSavingV2::from(&state));
    write(
        output,
        "populated-platform-state-v2.hex",
        bincode::encode_to_vec(stored, config).unwrap(),
    );
}

fn write(output: &Path, name: &str, bytes: Vec<u8>) {
    println!("{name}: {} bytes", bytes.len());
    std::fs::write(output.join(name), format!("{}\n", hex::encode(bytes))).unwrap();
}
