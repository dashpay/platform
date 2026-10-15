//! Operator keys that Dash Core re-encodes, run through the masternode list update as Drive runs
//! it once per block.
//!
//! The fixtures under `core_v24_protx_list_diffs/legacy_operator_key` are `protx listdiff` output
//! recorded from Dash Core v24.0.0-rc.3 on regtest. Four regular masternodes are registered with
//! legacy-scheme operator keys, and DEPLOYMENT_V24 activates at Core height 400. Then, one step per
//! Core block: MN_L1's operator sends a ProUpServTx (401) and MN_L2's owner a ProUpRegTx with a new
//! voting address (402), both keeping their operator key, which Core re-encodes in the basic
//! scheme; MN_L3's owner sets another operator key (403); MN_L4's operator revokes (404).
//! `399-404.json` is the same steps as one diff.

use crate::config::PlatformConfig;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey, SecretKey as BlsSecretKey};
use dpp::dashcore::address::NetworkUnchecked;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{Address, ProTxHash};
use dpp::dashcore_rpc::json::{DMNStateDiff, MasternodeListDiff, MasternodeListItem};
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{KeyType, Purpose};
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

/// The first Core height of the recording where DEPLOYMENT_V24 is active.
const V24_ACTIVATION_HEIGHT: u32 = 400;

pub(super) const MN_L1: &str = "f18c25367cd4f2ba6e8cdf86d0ffcf2694b6206b75597fe1ab5b80b966dbdf4b";
pub(super) const MN_L1_LEGACY_KEY: &str = "06ebb5ebd9079d73577e0900c9c10139d7d7213247e32b9b8823cd80e6a18e16a497ac57508e23b4a0e0510bfc880ad9";
pub(super) const MN_L1_BASIC_KEY: &str = "86ebb5ebd9079d73577e0900c9c10139d7d7213247e32b9b8823cd80e6a18e16a497ac57508e23b4a0e0510bfc880ad9";
pub(super) const MN_L2: &str = "02cc32acbd70020b192abdddfd8ab6d682b0e75c6cbe225f60fe6cf9da08859b";
pub(super) const MN_L2_LEGACY_KEY: &str = "910633a672392e26860cbff612b28d84f71beb34c2092d015b0d522905d628d38d746c1088c654632c65c6a7f008095d";
pub(super) const MN_L2_BASIC_KEY: &str = "b10633a672392e26860cbff612b28d84f71beb34c2092d015b0d522905d628d38d746c1088c654632c65c6a7f008095d";
pub(super) const MN_L3: &str = "192a79c8bcd46a6f8500b3010d6f4145a6580a0a0294500dfbb96dbd4c2dd625";
pub(super) const MN_L3_LEGACY_KEY: &str = "8e03b290379bfc0b6632e4eff4f45f40ff6888b49f0bfbca53f6e4f452c56b56a7c4f133dab94b0f61c05f3077317a9c";
pub(super) const MN_L3_NEW_KEY: &str = "ac4121f7c123b5e265d59053f5988a4d2677a794f2ab8b9147b88436b47c690fdd44af7868ca9047199c346c9323b3bf";
pub(super) const MN_L4: &str = "802390501809bee03accfa3742d62018a550324a918b85c6d106a51d946e8f2f";
pub(super) const MN_L4_LEGACY_KEY: &str = "0c718ddf5a8e115c2a4ef8bf906e89a69cd54b29405010ca16047581d92f382835ca6bb558a9ee237ed2879e312a3eb9";

/// MN_L2's voting address before and after its ProUpRegTx.
const MN_L2_VOTING_ADDRESS: &str = "yhUTNhZjweRvHEJ43JUefpTuHavmDh3LjP";
const MN_L2_NEW_VOTING_ADDRESS: &str = "yd59MzNMSWf5LdNE9qxZGrcFeR2BzjuqM2";

pub(super) fn bytes(hex: &str) -> Vec<u8> {
    hex::decode(hex).expect("expected hex")
}

pub(super) fn pro_tx_hash(hex: &str) -> ProTxHash {
    ProTxHash::from_str(hex).expect("expected a pro tx hash")
}

/// A recorded `protx listdiff` from `base_height` to `core_height`.
pub(super) fn recorded_list_diff(base_height: u32, core_height: u32) -> MasternodeListDiff {
    let json = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/supporting_files/core_v24_protx_list_diffs/legacy_operator_key")
            .join(format!("{base_height}-{core_height}.json")),
    )
    .expect("expected to read the masternode list diff");
    serde_json::from_str(&json).expect("expected Core's masternode list diff")
}

/// The recorded masternodes as Core lists them before V24, with their legacy-scheme keys.
pub(super) fn recorded_masternodes() -> BTreeMap<ProTxHash, MasternodeListItem> {
    recorded_list_diff(1, V24_ACTIVATION_HEIGHT - 1)
        .added_mns
        .into_iter()
        .map(|masternode| (masternode.pro_tx_hash, masternode))
        .collect()
}

/// A diff entry as Core prints it, with only the fields in `fields`.
pub(super) fn diff_entry(fields: serde_json::Value) -> DMNStateDiff {
    serde_json::from_value(fields).expect("expected a diff entry")
}

/// A basic-scheme public key whose sign bit is clear, and its negation. The 48 bytes of the first
/// also decode in the legacy scheme, to the second.
pub(super) fn key_and_negation(seed: u64) -> (Vec<u8>, Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(seed);
    loop {
        let key = BlsSecretKey::<Bls12381G2Impl>::random(&mut rng).public_key();
        let basic = key.to_bytes();
        if basic[0] & 0x20 == 0 {
            return (basic, BlsPublicKey::<Bls12381G2Impl>(-key.0).to_bytes());
        }
    }
}

/// A platform whose Core reports `list_diffs` by Core height, with coinbase payload version 4
/// from `v24_activation_height` on and version 3 before, as Core requires.
fn platform_with_core(
    platform_version: &PlatformVersion,
    list_diffs: BTreeMap<u32, MasternodeListDiff>,
    v24_activation_height: u32,
) -> TempPlatform<MockCoreRPCLike> {
    let mut platform = TestPlatformBuilder::new()
        .with_config(PlatformConfig::default())
        .with_initial_protocol_version(platform_version.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    platform
        .core_rpc
        .expect_get_protx_diff_with_masternodes()
        .returning(move |_, core_height| {
            Ok(list_diffs
                .get(&core_height)
                .unwrap_or_else(|| panic!("no masternode list diff for core height {core_height}"))
                .clone())
        });
    platform
        .core_rpc
        .expect_get_coinbase_payload_version()
        .returning(move |core_height| {
            Ok(Some(if core_height >= v24_activation_height {
                4
            } else {
                3
            }))
        });
    platform
}

/// The recorded diffs one Core height at a time, from the list at 399 to MN_L4's revocation.
fn recorded_steps() -> BTreeMap<u32, MasternodeListDiff> {
    let mut steps = BTreeMap::from([(
        V24_ACTIVATION_HEIGHT - 1,
        recorded_list_diff(1, V24_ACTIVATION_HEIGHT - 1),
    )]);
    for core_height in V24_ACTIVATION_HEIGHT..=V24_ACTIVATION_HEIGHT + 4 {
        steps.insert(
            core_height,
            recorded_list_diff(core_height - 1, core_height),
        );
    }
    steps
}

/// Runs one masternode list update per Core height of `list_diffs`, in order, as Drive runs one
/// per block, the first one from scratch as at init chain. With `reload_after`, the platform state
/// is stored and fetched back after that height, as a restarted node continues. Returns the state
/// after the last update.
fn run_core_heights(
    platform: &TempPlatform<MockCoreRPCLike>,
    transaction: &Transaction,
    platform_version: &PlatformVersion,
    core_heights: impl IntoIterator<Item = u32>,
    reload_after: Option<u32>,
) -> PlatformState {
    let mut committed: Option<PlatformState> = None;
    let mut block_state = platform.state.load().as_ref().clone();
    for (index, core_height) in core_heights.into_iter().enumerate() {
        let block_height = index as u64 + 1;
        platform
            .update_masternode_list(
                committed.as_ref(),
                &mut block_state,
                core_height,
                index == 0,
                &BlockInfo {
                    height: block_height,
                    core_height,
                    time_ms: block_height,
                    ..Default::default()
                },
                transaction,
                platform_version,
            )
            .expect("expected to apply Core's masternode list diff");
        if reload_after == Some(core_height) {
            platform
                .store_platform_state(&block_state, Some(transaction), platform_version)
                .expect("expected to store the platform state");
            block_state = Platform::<MockCoreRPCLike>::fetch_platform_state(
                &platform.drive,
                Some(transaction),
                platform_version,
            )
            .expect("expected to fetch the platform state")
            .expect("expected a stored platform state");
        }
        committed = Some(block_state.clone());
    }
    block_state
}

/// The (key type, purpose, data, disabled) of every key of an identity; `None` without one.
fn identity_keys(
    platform: &TempPlatform<MockCoreRPCLike>,
    transaction: &Transaction,
    platform_version: &PlatformVersion,
    identity_id: Identifier,
) -> Option<Vec<(KeyType, Purpose, Vec<u8>, bool)>> {
    platform
        .drive
        .fetch_full_identity(identity_id.to_buffer(), Some(transaction), platform_version)
        .expect("expected to fetch an identity")
        .map(|identity| {
            identity
                .public_keys()
                .values()
                .map(|key| {
                    (
                        key.key_type(),
                        key.purpose(),
                        key.data().to_vec(),
                        key.is_disabled(),
                    )
                })
                .collect()
        })
}

fn operator_identifier(pro_tx_hash: &ProTxHash, key: &[u8]) -> Identifier {
    Identifier::create_operator_identifier(&pro_tx_hash.to_byte_array(), key)
}

/// Whether the identity has a BLS key with `data`, and whether that key is disabled.
fn bls_key_disabled(keys: &[(KeyType, Purpose, Vec<u8>, bool)], data: &[u8]) -> Option<bool> {
    keys.iter()
        .find(|(key_type, _, key_data, _)| *key_type == KeyType::BLS12_381 && key_data == data)
        .map(|(_, _, _, disabled)| *disabled)
}

fn voting_key_hash(address: &str) -> [u8; 20] {
    Address::<NetworkUnchecked>::from_str(address)
        .expect("expected an address")
        .payload_to_vec()
        .try_into()
        .expect("expected a 20-byte key hash")
}

/// Once V24 is active, Core re-encodes a legacy-scheme operator key in the basic scheme when the
/// masternode's state version is raised, and reports the new bytes as a changed key. The key is
/// the same, so the masternode keeps its operator identity: the identity derived from the stored
/// legacy bytes keeps its keys enabled, the list keeps those bytes, and no identity is created for
/// the basic bytes. The other fields of the same entry apply as always, and in the same diff a
/// real key change and a revocation switch operator identities as always. Checked with the
/// recorded steps one Core height per block, and with all of them in one diff.
#[test]
fn should_keep_the_operator_identity_of_a_legacy_key_core_re_encodes() {
    for platform_version in [
        PlatformVersion::get(13).expect("expected protocol version 13"),
        PlatformVersion::latest(),
    ] {
        let version = platform_version.protocol_version;
        let one_window = BTreeMap::from([
            (
                V24_ACTIVATION_HEIGHT - 1,
                recorded_list_diff(1, V24_ACTIVATION_HEIGHT - 1),
            ),
            (
                V24_ACTIVATION_HEIGHT + 4,
                recorded_list_diff(V24_ACTIVATION_HEIGHT - 1, V24_ACTIVATION_HEIGHT + 4),
            ),
        ]);
        for (windows, list_diffs) in [
            ("one per Core height", recorded_steps()),
            ("one diff", one_window),
        ] {
            let core_heights: Vec<u32> = list_diffs.keys().copied().collect();
            let platform = platform_with_core(platform_version, list_diffs, V24_ACTIVATION_HEIGHT);
            let transaction = platform.drive.grove.start_transaction();
            let state = run_core_heights(
                &platform,
                &transaction,
                platform_version,
                core_heights,
                None,
            );
            let keys =
                |identity_id| identity_keys(&platform, &transaction, platform_version, identity_id);
            let context = format!("PV{version}, {windows}");

            for (masternode, legacy_key, basic_key) in [
                (MN_L1, MN_L1_LEGACY_KEY, MN_L1_BASIC_KEY),
                (MN_L2, MN_L2_LEGACY_KEY, MN_L2_BASIC_KEY),
            ] {
                let masternode = pro_tx_hash(masternode);
                assert_eq!(
                    state.full_masternode_list()[&masternode]
                        .state
                        .pub_key_operator,
                    bytes(legacy_key),
                    "{context}: the stored operator key of {masternode} keeps its legacy bytes"
                );
                let operator_keys = keys(operator_identifier(&masternode, &bytes(legacy_key)))
                    .expect("expected the operator identity derived from the legacy bytes");
                assert!(
                    operator_keys.iter().all(|(.., disabled)| !disabled),
                    "{context}: every key of the operator identity of {masternode} stays enabled: {operator_keys:?}"
                );
                assert_eq!(
                    bls_key_disabled(&operator_keys, &bytes(legacy_key)),
                    Some(false),
                    "{context}: {masternode}"
                );
                assert_eq!(
                    keys(operator_identifier(&masternode, &bytes(basic_key))),
                    None,
                    "{context}: no operator identity for the re-encoded bytes of {masternode}"
                );
            }

            // MN_L2 keeps its operator payout key, and the new voting address of the same
            // ProUpRegTx is applied, with its voter identity.
            let mn_l2 = pro_tx_hash(MN_L2);
            assert!(
                keys(operator_identifier(&mn_l2, &bytes(MN_L2_LEGACY_KEY)))
                    .expect("expected MN_L2's operator identity")
                    .iter()
                    .any(|(_, purpose, _, disabled)| *purpose == Purpose::TRANSFER && !disabled),
                "{context}: MN_L2 keeps its operator payout key"
            );
            let new_voting_key_hash = voting_key_hash(MN_L2_NEW_VOTING_ADDRESS);
            assert_ne!(new_voting_key_hash, voting_key_hash(MN_L2_VOTING_ADDRESS));
            assert_eq!(
                state.full_masternode_list()[&mn_l2].state.voting_address,
                new_voting_key_hash,
                "{context}: MN_L2's new voting address is applied"
            );
            assert!(
                keys(Identifier::create_voter_identifier(
                    &mn_l2.to_byte_array(),
                    &new_voting_key_hash,
                ))
                .is_some(),
                "{context}: MN_L2's new voting address has its voter identity"
            );

            // MN_L3 set another operator key: the operator identity switches as always.
            let mn_l3 = pro_tx_hash(MN_L3);
            assert_eq!(
                state.full_masternode_list()[&mn_l3].state.pub_key_operator,
                bytes(MN_L3_NEW_KEY),
                "{context}"
            );
            assert_eq!(
                bls_key_disabled(
                    &keys(operator_identifier(&mn_l3, &bytes(MN_L3_LEGACY_KEY)))
                        .expect("expected MN_L3's previous operator identity"),
                    &bytes(MN_L3_LEGACY_KEY)
                ),
                Some(true),
                "{context}: MN_L3's previous operator key is disabled"
            );
            assert_eq!(
                bls_key_disabled(
                    &keys(operator_identifier(&mn_l3, &bytes(MN_L3_NEW_KEY)))
                        .expect("expected MN_L3's new operator identity"),
                    &bytes(MN_L3_NEW_KEY)
                ),
                Some(false),
                "{context}"
            );

            // MN_L4's operator revoked: Core reports the null key, a key change as always.
            assert_eq!(
                bls_key_disabled(
                    &keys(operator_identifier(
                        &pro_tx_hash(MN_L4),
                        &bytes(MN_L4_LEGACY_KEY)
                    ))
                    .expect("expected MN_L4's previous operator identity"),
                    &bytes(MN_L4_LEGACY_KEY)
                ),
                Some(true),
                "{context}: MN_L4's revoked operator key is disabled"
            );
        }
    }
}

/// Platform keeps a re-encoded key only where Core produces one, which Core's coinbase marks:
/// with the coinbase payload version below 4 at the block's Core height, the same recorded diff
/// is a key change, as it always was.
#[test]
fn should_switch_the_operator_identity_on_a_re_encoded_key_while_v24_is_not_active() {
    for platform_version in [
        PlatformVersion::get(13).expect("expected protocol version 13"),
        PlatformVersion::latest(),
    ] {
        let version = platform_version.protocol_version;
        let platform = platform_with_core(platform_version, recorded_steps(), u32::MAX);
        let transaction = platform.drive.grove.start_transaction();
        let state = run_core_heights(
            &platform,
            &transaction,
            platform_version,
            V24_ACTIVATION_HEIGHT - 1..=V24_ACTIVATION_HEIGHT + 1,
            None,
        );
        let mn_l1 = pro_tx_hash(MN_L1);
        assert_eq!(
            state.full_masternode_list()[&mn_l1].state.pub_key_operator,
            bytes(MN_L1_BASIC_KEY),
            "PV{version}"
        );
        assert_eq!(
            bls_key_disabled(
                &identity_keys(
                    &platform,
                    &transaction,
                    platform_version,
                    operator_identifier(&mn_l1, &bytes(MN_L1_LEGACY_KEY))
                )
                .expect("expected MN_L1's previous operator identity"),
                &bytes(MN_L1_LEGACY_KEY)
            ),
            Some(true),
            "PV{version}"
        );
        assert!(identity_keys(
            &platform,
            &transaction,
            platform_version,
            operator_identifier(&mn_l1, &bytes(MN_L1_BASIC_KEY))
        )
        .is_some());
    }
}

/// After the recorded steps, MN_L2's owner sets another operator key and MN_L1 is removed. Both
/// reach the operator identity kept on the re-encode: the key change disables its BLS key and
/// creates the new identity, the removal disables its BLS key. A node that stored and reloaded its
/// platform state after the first re-encode ends with the same state as one that did not.
#[test]
fn should_reach_the_kept_operator_identity_after_a_re_encode_and_a_restart() {
    let mn_l2_new_key = key_and_negation(11).0;
    let after_recording = |list_diffs: &mut BTreeMap<u32, MasternodeListDiff>| {
        let key_change_height = V24_ACTIVATION_HEIGHT + 5;
        list_diffs.insert(
            key_change_height,
            MasternodeListDiff {
                base_height: key_change_height - 1,
                block_height: key_change_height,
                added_mns: vec![],
                removed_mns: vec![],
                updated_mns: vec![(
                    pro_tx_hash(MN_L2),
                    diff_entry(json!({ "pubKeyOperator": hex::encode(&mn_l2_new_key) })),
                )],
            },
        );
        list_diffs.insert(
            key_change_height + 1,
            MasternodeListDiff {
                base_height: key_change_height,
                block_height: key_change_height + 1,
                added_mns: vec![],
                removed_mns: vec![pro_tx_hash(MN_L1)],
                updated_mns: vec![],
            },
        );
    };
    for platform_version in [
        PlatformVersion::get(13).expect("expected protocol version 13"),
        PlatformVersion::latest(),
    ] {
        let version = platform_version.protocol_version;
        let mut root_hashes = vec![];
        for reload_after in [None, Some(V24_ACTIVATION_HEIGHT + 1)] {
            let mut list_diffs = recorded_steps();
            after_recording(&mut list_diffs);
            let core_heights: Vec<u32> = list_diffs.keys().copied().collect();
            let platform = platform_with_core(platform_version, list_diffs, V24_ACTIVATION_HEIGHT);
            let transaction = platform.drive.grove.start_transaction();
            let state = run_core_heights(
                &platform,
                &transaction,
                platform_version,
                core_heights,
                reload_after,
            );
            let keys =
                |identity_id| identity_keys(&platform, &transaction, platform_version, identity_id);
            let context = format!("PV{version}, reload after {reload_after:?}");

            let mn_l2 = pro_tx_hash(MN_L2);
            assert_eq!(
                bls_key_disabled(
                    &keys(operator_identifier(&mn_l2, &bytes(MN_L2_LEGACY_KEY)))
                        .expect("expected MN_L2's kept operator identity"),
                    &bytes(MN_L2_LEGACY_KEY)
                ),
                Some(true),
                "{context}: MN_L2's key change disables the BLS key of the identity it kept"
            );
            assert_eq!(
                bls_key_disabled(
                    &keys(operator_identifier(&mn_l2, &mn_l2_new_key))
                        .expect("expected MN_L2's new operator identity"),
                    &mn_l2_new_key
                ),
                Some(false),
                "{context}"
            );

            let mn_l1 = pro_tx_hash(MN_L1);
            assert!(!state.full_masternode_list().contains_key(&mn_l1));
            assert_eq!(
                bls_key_disabled(
                    &keys(operator_identifier(&mn_l1, &bytes(MN_L1_LEGACY_KEY)))
                        .expect("expected MN_L1's operator identity"),
                    &bytes(MN_L1_LEGACY_KEY)
                ),
                Some(true),
                "{context}: removing MN_L1 disables the BLS key of the identity it kept"
            );
            assert_eq!(
                keys(operator_identifier(&mn_l1, &bytes(MN_L1_BASIC_KEY))),
                None,
                "{context}: MN_L1 never had a second operator identity"
            );
            root_hashes.push(
                platform
                    .drive
                    .grove
                    .root_hash(Some(&transaction), &platform_version.drive.grove_version)
                    .value
                    .expect("expected a root hash"),
            );
        }
        assert_eq!(
            root_hashes[0], root_hashes[1],
            "PV{version}: a restart after the re-encode does not change the outcome"
        );
    }
}

/// The masternode for the synthetic steps: MN_L1 as recorded, with operator key `key`.
fn masternode_with_key(key: Vec<u8>) -> MasternodeListItem {
    let mut masternode = recorded_masternodes()
        .remove(&pro_tx_hash(MN_L1))
        .expect("expected MN_L1 in the recording");
    masternode.state.pub_key_operator = key;
    masternode
}

/// Core reports, one Core height each from 11, operator key `steps` for MN_L1 listed at 10 with
/// operator key `key`; V24 is active from `v24_activation_height`. Returns the platform, with the
/// updates committed, and the state after the last step.
fn run_operator_key_steps(
    platform_version: &PlatformVersion,
    key: Vec<u8>,
    steps: Vec<Vec<u8>>,
    v24_activation_height: u32,
) -> (TempPlatform<MockCoreRPCLike>, PlatformState) {
    let listed = masternode_with_key(key);
    let mut list_diffs = BTreeMap::from([(
        10,
        MasternodeListDiff {
            base_height: 1,
            block_height: 10,
            added_mns: vec![listed.clone()],
            removed_mns: vec![],
            updated_mns: vec![],
        },
    )]);
    for (core_height, step) in (11..).zip(steps) {
        list_diffs.insert(
            core_height,
            MasternodeListDiff {
                base_height: core_height - 1,
                block_height: core_height,
                added_mns: vec![],
                removed_mns: vec![],
                updated_mns: vec![(
                    listed.pro_tx_hash,
                    diff_entry(json!({ "pubKeyOperator": hex::encode(step) })),
                )],
            },
        );
    }
    let core_heights: Vec<u32> = list_diffs.keys().copied().collect();
    let platform = platform_with_core(platform_version, list_diffs, v24_activation_height);
    let transaction = platform.drive.grove.start_transaction();
    let state = run_core_heights(
        &platform,
        &transaction,
        platform_version,
        core_heights,
        None,
    );
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit");
    (platform, state)
}

/// Before V24 Platform applies every operator key Core reports exactly as it always has, so the
/// blocks it has executed replay to the same result. That holds for the keys the rules for
/// re-encoded keys would otherwise keep: one whose bytes read in the legacy scheme give the stored
/// key's negation, and one whose bytes equal the stored ones.
#[test]
fn should_apply_every_reported_operator_key_before_v24() {
    for platform_version in [
        PlatformVersion::get(13).expect("expected protocol version 13"),
        PlatformVersion::latest(),
    ] {
        let version = platform_version.protocol_version;
        let (key, negation) = key_and_negation(7);
        let mn_l1 = pro_tx_hash(MN_L1);

        let (platform, state) =
            run_operator_key_steps(platform_version, key.clone(), vec![negation.clone()], 100);
        let transaction = platform.drive.grove.start_transaction();
        let keys =
            |identity_id| identity_keys(&platform, &transaction, platform_version, identity_id);
        assert_eq!(
            state.full_masternode_list()[&mn_l1].state.pub_key_operator,
            negation,
            "PV{version}"
        );
        assert_eq!(
            bls_key_disabled(
                &keys(operator_identifier(&mn_l1, &key)).expect("expected the previous identity"),
                &key
            ),
            Some(true),
            "PV{version}: the previous operator key is disabled"
        );
        assert!(keys(operator_identifier(&mn_l1, &negation)).is_some());

        let (platform, _) =
            run_operator_key_steps(platform_version, key.clone(), vec![key.clone()], 100);
        let transaction = platform.drive.grove.start_transaction();
        assert_eq!(
            bls_key_disabled(
                &identity_keys(
                    &platform,
                    &transaction,
                    platform_version,
                    operator_identifier(&mn_l1, &key)
                )
                .expect("expected the operator identity"),
                &key
            ),
            Some(true),
            "PV{version}: a reported key equal to the stored one is applied as a key change"
        );
    }
}

/// Once V24 is active at the block's Core height, a reported key whose bytes are the basic
/// encoding of the stored bytes read in the legacy scheme keeps the operator identity, and so does
/// a later key whose bytes equal the stored bytes. Such a pair is a key and its negation: whoever
/// holds the secret of one holds the secret of the other. V24 activates at the first step, so
/// the Core height asked about is the block's own.
#[test]
fn should_keep_the_operator_identity_for_a_negated_key_once_v24_is_active() {
    for platform_version in [
        PlatformVersion::get(13).expect("expected protocol version 13"),
        PlatformVersion::latest(),
    ] {
        let version = platform_version.protocol_version;
        let (key, negation) = key_and_negation(7);
        let mn_l1 = pro_tx_hash(MN_L1);
        let (platform, state) = run_operator_key_steps(
            platform_version,
            key.clone(),
            vec![negation.clone(), key.clone()],
            11,
        );
        let transaction = platform.drive.grove.start_transaction();
        assert_eq!(
            state.full_masternode_list()[&mn_l1].state.pub_key_operator,
            key,
            "PV{version}: the stored key keeps its bytes"
        );
        let operator_keys = identity_keys(
            &platform,
            &transaction,
            platform_version,
            operator_identifier(&mn_l1, &key),
        )
        .expect("expected the operator identity");
        assert!(
            operator_keys.iter().all(|(.., disabled)| !disabled),
            "PV{version}: {operator_keys:?}"
        );
        assert_eq!(
            identity_keys(
                &platform,
                &transaction,
                platform_version,
                operator_identifier(&mn_l1, &negation)
            ),
            None,
            "PV{version}"
        );
    }
}
