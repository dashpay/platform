//! The first block of protocol version 15 reconciles the operator payouts Platform holds with
//! Dash Core's masternode list, at the last committed chain locked core height.

use crate::config::PlatformConfig;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::dash_to_credits;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{ProTxHash, Txid};
use dpp::dashcore_rpc::dashcore_rpc_json::{
    DMNState, DMNStateDiff, ExtendedQuorumListResult, MasternodeListDiff, MasternodeListItem,
    MasternodeType,
};
use dpp::fee::Credits;
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::identity_credit_withdrawal_transition::methods::{
    IdentityCreditWithdrawalTransitionMethodsV0, PreferredKeyPurposeForSigningWithdrawal,
};
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::version::{PlatformVersion, ProtocolVersion};
use dpp::withdrawal::Pooling;
use drive::grovedb::Transaction;
use rand::rngs::StdRng;
use rand::SeedableRng;
use simple_signer::signer::SimpleSigner;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// The Core height the masternodes are first listed at, and the last committed one, which the
/// upgrade block reconciles at.
const LISTED_AT: u32 = 999;
const COMMITTED_AT: u32 = 1000;
/// The core height the upgrade block moves to.
const UPGRADE_AT: u32 = 1001;

/// A Regular masternode named by `id` with an operator reward, operator key `[key; 48]` and
/// `operator_payout`.
fn masternode(id: u8, key: u8, operator_payout: Option<u8>) -> MasternodeListItem {
    MasternodeListItem {
        node_type: MasternodeType::Regular,
        pro_tx_hash: ProTxHash::from_byte_array([id; 32]),
        collateral_hash: Txid::from_byte_array([id ^ 0x40; 32]),
        collateral_index: 0,
        collateral_address: Some([id ^ 0x50; 20]),
        operator_reward: 10.0,
        state: DMNState {
            service: SocketAddr::from(([192, 0, 2, id], 9999)),
            registered_height: 900,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: 0,
            owner_address: Some([id ^ 0x10; 20]),
            voting_address: [id ^ 0x20; 20],
            payout_address: Some([id ^ 0x30; 20]),
            payouts: None,
            pub_key_operator: vec![key; 48],
            operator_payout_address: operator_payout.map(|payout| [payout; 20]),
            platform_node_id: None,
            #[allow(deprecated)]
            legacy_platform_p2p_port: None,
            #[allow(deprecated)]
            legacy_platform_http_port: None,
            addresses: None,
        },
    }
}

fn diff_entry(fields: serde_json::Value) -> DMNStateDiff {
    serde_json::from_value(fields).expect("expected a Core state diff")
}

/// The masternode list calls Platform made to Core, as (base height, height).
type CoreCalls = Arc<Mutex<Vec<(Option<u32>, u32)>>>;

/// Three masternodes, as Core lists them at `LISTED_AT` and at `COMMITTED_AT`, and the diff Core
/// prints between the two: 0x61's operator sets another payout in place of 0x61, and 0x62's
/// owner sets operator key 0x72, which empties the payout 0x62. 0x64 does not change.
fn core_history() -> (
    Vec<MasternodeListItem>,
    MasternodeListDiff,
    Vec<MasternodeListItem>,
) {
    let listed = vec![
        masternode(0x61, 0x61, Some(0x61)),
        masternode(0x62, 0x62, Some(0x62)),
        masternode(0x64, 0x64, Some(0x64)),
    ];
    let diff = MasternodeListDiff {
        base_height: LISTED_AT,
        block_height: COMMITTED_AT,
        added_mns: vec![],
        removed_mns: vec![],
        updated_mns: vec![
            (
                listed[0].pro_tx_hash,
                diff_entry(serde_json::json!({
                    "operatorPayoutAddress": "yXkZa7p2ne58x2XQYdNthAxfXy9cHiWxF5",
                })),
            ),
            (
                listed[1].pro_tx_hash,
                diff_entry(serde_json::json!({ "pubKeyOperator": "72".repeat(48) })),
            ),
        ],
    };
    let mut at_committed = listed.clone();
    at_committed[0].state.operator_payout_address = diff.updated_mns[0]
        .1
        .operator_payout_address
        .expect("expected the diff to set the payout");
    at_committed[1].state.pub_key_operator = vec![0x72; 48];
    at_committed[1].state.operator_payout_address = None;
    (listed, diff, at_committed)
}

/// The diff of the upgrade block itself, `COMMITTED_AT` to `UPGRADE_AT`: 0x61's owner sets
/// operator key 0x71, 0x62's new operator sets a payout, 0x64 is removed and 0x65 registered.
fn upgrade_block_diff() -> MasternodeListDiff {
    MasternodeListDiff {
        base_height: COMMITTED_AT,
        block_height: UPGRADE_AT,
        added_mns: vec![masternode(0x65, 0x65, Some(0x65))],
        removed_mns: vec![ProTxHash::from_byte_array([0x64; 32])],
        updated_mns: vec![
            (
                ProTxHash::from_byte_array([0x61; 32]),
                diff_entry(serde_json::json!({ "pubKeyOperator": "71".repeat(48) })),
            ),
            (
                ProTxHash::from_byte_array([0x62; 32]),
                diff_entry(serde_json::json!({
                    "operatorPayoutAddress": "yd59MzNMSWf5LdNE9qxZGrcFeR2BzjuqM2",
                })),
            ),
        ],
    }
}

/// Runs the protocol change events of the first block of protocol version 15 after `previous`,
/// in `transaction`, on a fresh copy of `committed` as `run_block_proposal` makes for each round.
fn run_upgrade_events(
    platform: &TempPlatform<MockCoreRPCLike>,
    committed: &PlatformState,
    previous: ProtocolVersion,
    transaction: &Transaction,
) -> PlatformState {
    let platform_version = PlatformVersion::latest();
    let mut block_state = committed.clone();
    block_state.set_current_protocol_version_in_consensus(platform_version.protocol_version);
    platform
        .perform_events_on_first_block_of_protocol_change(
            committed,
            &mut block_state,
            &BlockInfo {
                height: 3,
                core_height: UPGRADE_AT,
                ..Default::default()
            },
            transaction,
            previous,
            platform_version,
        )
        .expect("expected the protocol change events to succeed");
    block_state
}

/// A platform at `previous` that processed `core_history` as Platform did at that protocol
/// version, through `update_core_info`: the list at `LISTED_AT` at init chain, then the diff in a
/// block at `COMMITTED_AT`.
/// Returns the committed state and the masternode list calls Core gets after that block.
fn platform_after_core_history(
    previous: ProtocolVersion,
) -> (TempPlatform<MockCoreRPCLike>, PlatformState, CoreCalls) {
    let (listed, diff, at_committed) = core_history();
    platform_after_history(previous, listed, vec![diff], at_committed)
}

/// A platform at `previous` that processed, through `update_core_info`, Core's list `listed` at
/// init chain and then each of `diffs` in a block of its own, one Core height each, the last at
/// `COMMITTED_AT`. Core lists `at_committed` at `COMMITTED_AT` and prints `upgrade_block_diff`
/// after it. Returns the committed state and the masternode list calls Core gets after that.
fn platform_after_history(
    previous: ProtocolVersion,
    listed: Vec<MasternodeListItem>,
    diffs: Vec<MasternodeListDiff>,
    at_committed: Vec<MasternodeListItem>,
) -> (TempPlatform<MockCoreRPCLike>, PlatformState, CoreCalls) {
    platform_with_config_after_history(
        PlatformConfig::default(),
        previous,
        listed,
        diffs,
        at_committed,
        upgrade_block_diff(),
    )
}

/// `platform_after_history` with `config`, and with Core printing `upgrade_diff` after
/// `COMMITTED_AT`.
fn platform_with_config_after_history(
    config: PlatformConfig,
    previous: ProtocolVersion,
    listed: Vec<MasternodeListItem>,
    diffs: Vec<MasternodeListDiff>,
    at_committed: Vec<MasternodeListItem>,
    upgrade_diff: MasternodeListDiff,
) -> (TempPlatform<MockCoreRPCLike>, PlatformState, CoreCalls) {
    let previous_version = PlatformVersion::get(previous).expect("expected the protocol version");
    let mut platform = TestPlatformBuilder::new()
        .with_config(config)
        .with_initial_protocol_version(previous)
        .build_with_mock_rpc()
        .set_genesis_state();
    let listed_at = COMMITTED_AT - diffs.len() as u32;
    let calls = CoreCalls::default();
    let recorded_calls = Arc::clone(&calls);
    let history_done = Arc::new(Mutex::new(false));
    let done = Arc::clone(&history_done);
    platform
        .core_rpc
        .expect_get_protx_diff_with_masternodes()
        .returning(move |base_block, block| {
            if *done.lock().expect("expected the history flag") {
                recorded_calls
                    .lock()
                    .expect("expected the Core calls")
                    .push((base_block, block));
            }
            let added_mns = match (base_block, block) {
                (None, block) if block == listed_at => listed.clone(),
                (None, COMMITTED_AT) => at_committed.clone(),
                (Some(COMMITTED_AT), UPGRADE_AT) => return Ok(upgrade_diff.clone()),
                (Some(base), block) if base >= listed_at && block == base + 1 => {
                    return Ok(diffs[(base - listed_at) as usize].clone())
                }
                other => panic!("unexpected masternode list call {other:?}"),
            };
            Ok(MasternodeListDiff {
                base_height: 1,
                block_height: block,
                added_mns,
                removed_mns: vec![],
                updated_mns: vec![],
            })
        });

    // Asked about a masternode whose operator key changed without a printed payout, Core reports
    // none, as after a key change that set none. Up to protocol version 14 nothing asks.
    let state_reads = Arc::new(Mutex::new(0));
    let recorded_state_reads = Arc::clone(&state_reads);
    platform
        .core_rpc
        .expect_get_masternode_state()
        .returning(move |_, _| {
            *recorded_state_reads
                .lock()
                .expect("expected the state reads") += 1;
            Ok(masternode(0, 0, None).state)
        });
    let quorum_types = [
        platform.config.validator_set.quorum_type,
        platform.config.chain_lock.quorum_type,
        platform.config.instant_lock.quorum_type,
    ];
    platform
        .core_rpc
        .expect_get_quorum_listextended()
        .returning(move |_| {
            Ok(ExtendedQuorumListResult {
                quorums_by_type: quorum_types
                    .iter()
                    .map(|quorum_type| (*quorum_type, Default::default()))
                    .collect::<HashMap<_, _>>(),
            })
        });

    let transaction = platform.drive.grove.start_transaction();
    let mut state = platform.state.load().as_ref().clone();
    let mut committed: Option<PlatformState> = None;
    for (height, core_height) in (1..).zip(listed_at..=COMMITTED_AT) {
        platform
            .update_core_info(
                committed.as_ref(),
                &mut state,
                core_height,
                committed.is_none(),
                &BlockInfo {
                    height,
                    core_height,
                    ..Default::default()
                },
                &transaction,
                previous_version,
            )
            .expect("expected to process Core's masternode list");
        state.set_last_committed_block_info(Some(committed_block(height, core_height)));
        committed = Some(state.clone());
    }
    state.mark_saved();
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit");
    *history_done.lock().expect("expected the history flag") = true;
    assert_eq!(
        *state_reads.lock().expect("expected the state reads"),
        0,
        "protocol version {previous} asked Core for a masternode's state"
    );
    (platform, state, calls)
}

fn committed_block(height: u64, core_height: u32) -> ExtendedBlockInfo {
    ExtendedBlockInfoV0 {
        basic_info: BlockInfo {
            height,
            core_height,
            ..Default::default()
        },
        app_hash: [0; 32],
        quorum_hash: [0; 32],
        block_id_hash: [0; 32],
        proposer_pro_tx_hash: [0; 32],
        signature: [0; 96],
        round: 0,
    }
    .into()
}

/// The (data, disabled) of every TRANSFER key of the operator identity of the masternode named
/// by `id` with operator key `[key; 48]`.
fn transfer_keys(
    platform: &TempPlatform<MockCoreRPCLike>,
    transaction: &Transaction,
    id: u8,
    key: u8,
) -> Vec<(Vec<u8>, bool)> {
    platform
        .drive
        .fetch_full_identity(
            Identifier::create_operator_identifier(&[id; 32], &[key; 48]).to_buffer(),
            Some(transaction),
            PlatformVersion::latest(),
        )
        .expect("expected to fetch the operator identity")
        .expect("expected the operator identity")
        .public_keys()
        .values()
        .filter(|key| key.purpose() == Purpose::TRANSFER)
        .map(|key| (key.data().to_vec(), key.is_disabled()))
        .collect()
}

/// Up to protocol version 14 a diff's operator payout never reached Platform, and a new operator
/// identity took the stored payout, the previous operator's. The first block of protocol version
/// 15, also after a jump from 13, reads Core's list at the last committed core height and fixes
/// both, in the block's state and its transaction: the changed payout's key replaces the old one,
/// and the new operator's identity loses the inherited key, which the previous operator's
/// identity keeps.
#[test]
fn should_reconcile_operator_payouts_with_core_in_the_first_block_of_protocol_version_15() {
    let platform_version = PlatformVersion::latest();
    for previous in [13, 14] {
        let (platform, committed, calls) = platform_after_core_history(previous);
        assert_eq!(
            committed.full_masternode_list()[&ProTxHash::from_byte_array([0x61; 32])]
                .state
                .operator_payout_address,
            Some([0x61; 20]),
            "PV{previous} kept the listed payout"
        );
        let transaction = platform.drive.grove.start_transaction();
        assert_eq!(
            transfer_keys(&platform, &transaction, 0x62, 0x72),
            [(vec![0x62; 20], false)],
            "PV{previous} gave the new operator the previous operator's payout key"
        );
        let mut block_state = committed.clone();
        block_state.set_current_protocol_version_in_consensus(platform_version.protocol_version);

        platform
            .perform_events_on_first_block_of_protocol_change(
                &committed,
                &mut block_state,
                &BlockInfo {
                    height: 3,
                    core_height: COMMITTED_AT + 1,
                    ..Default::default()
                },
                &transaction,
                previous,
                platform_version,
            )
            .expect("expected the protocol change events to succeed");

        let context = format!("from PV{previous}");
        assert_eq!(
            *calls.lock().unwrap(),
            vec![(None, COMMITTED_AT)],
            "{context}"
        );
        let stored_payout = |id: u8| {
            block_state.full_masternode_list()[&ProTxHash::from_byte_array([id; 32])]
                .state
                .operator_payout_address
        };
        let (_, _, at_committed) = core_history();
        assert_eq!(
            stored_payout(0x61),
            at_committed[0].state.operator_payout_address,
            "{context}"
        );
        assert_eq!(stored_payout(0x62), None, "{context}");
        let new_payout = at_committed[0]
            .state
            .operator_payout_address
            .expect("expected Core's payout")
            .to_vec();
        assert_eq!(
            transfer_keys(&platform, &transaction, 0x61, 0x61),
            [(vec![0x61; 20], true), (new_payout, false)],
            "{context}"
        );
        assert_eq!(
            transfer_keys(&platform, &transaction, 0x62, 0x72),
            [(vec![0x62; 20], true)],
            "{context}"
        );
        assert_eq!(
            transfer_keys(&platform, &transaction, 0x62, 0x62),
            [(vec![0x62; 20], false)],
            "{context}: the previous operator's identity keeps its payout key"
        );
        assert_eq!(
            transfer_keys(&platform, &transaction, 0x64, 0x64),
            [(vec![0x64; 20], false)],
            "{context}: an identity that already matches Core is left as it is"
        );
        assert_eq!(stored_payout(0x64), Some([0x64; 20]), "{context}");
        assert!(
            !block_state
                .masternode_changes
                .upserted
                .contains(&ProTxHash::from_byte_array([0x64; 32])),
            "{context}: a masternode whose payout matches Core's is not rewritten"
        );
    }
}

/// Only the upgrade to protocol version 15 reconciles: the first block of 14 does not read Core's
/// masternode list, and leaves the payouts as they are.
#[test]
fn should_not_reconcile_operator_payouts_in_the_first_block_of_protocol_version_14() {
    let (platform, committed, calls) = platform_after_core_history(13);
    let transaction = platform.drive.grove.start_transaction();
    let mut block_state = committed.clone();
    block_state.set_current_protocol_version_in_consensus(14);

    platform
        .perform_events_on_first_block_of_protocol_change(
            &committed,
            &mut block_state,
            &BlockInfo {
                height: 3,
                core_height: COMMITTED_AT + 1,
                ..Default::default()
            },
            &transaction,
            13,
            PlatformVersion::get(14).expect("expected protocol version 14"),
        )
        .expect("expected the protocol change events to succeed");

    assert_eq!(*calls.lock().unwrap(), vec![]);
    assert_eq!(
        block_state.full_masternode_list(),
        committed.full_masternode_list()
    );
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x62, 0x72),
        [(vec![0x62; 20], false)]
    );
}

/// The upgrade block then applies its own diff from the reconciled list, under protocol version
/// 15's rules, with the committed state as the identity updates' base as in a real block: a key
/// change takes Core's payout (none here), a payout reaches the identity the reconciliation
/// fixed, under a fresh key id, and masternodes come and go as always.
#[test]
fn should_apply_the_upgrade_blocks_own_diff_after_the_reconciliation() {
    let platform_version = PlatformVersion::latest();
    let (platform, committed, calls) = platform_after_core_history(14);
    let transaction = platform.drive.grove.start_transaction();
    let mut block_state = run_upgrade_events(&platform, &committed, 14, &transaction);

    platform
        .update_core_info(
            Some(&committed),
            &mut block_state,
            UPGRADE_AT,
            false,
            &BlockInfo {
                height: 3,
                core_height: UPGRADE_AT,
                ..Default::default()
            },
            &transaction,
            platform_version,
        )
        .expect("expected to apply the upgrade block's diff");

    assert_eq!(
        *calls.lock().unwrap(),
        vec![(None, COMMITTED_AT), (Some(COMMITTED_AT), UPGRADE_AT)]
    );
    let stored = |id: u8| {
        block_state
            .full_masternode_list()
            .get(&ProTxHash::from_byte_array([id; 32]))
            .map(|masternode| {
                (
                    masternode.state.pub_key_operator.clone(),
                    masternode.state.operator_payout_address,
                )
            })
    };
    assert_eq!(stored(0x61), Some((vec![0x71; 48], None)));
    let upgrade_diff = upgrade_block_diff();
    let new_payout = upgrade_diff.updated_mns[1]
        .1
        .operator_payout_address
        .expect("expected the diff to set the payout")
        .expect("expected a payout");
    assert_eq!(stored(0x62), Some((vec![0x72; 48], Some(new_payout))));
    assert_eq!(stored(0x64), None);
    assert_eq!(stored(0x65), Some((vec![0x65; 48], Some([0x65; 20]))));

    assert_eq!(transfer_keys(&platform, &transaction, 0x61, 0x71), []);
    let (_, _, at_committed) = core_history();
    let reconciled_payout = at_committed[0]
        .state
        .operator_payout_address
        .expect("expected Core's payout")
        .to_vec();
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x61, 0x61),
        [(vec![0x61; 20], true), (reconciled_payout, false)],
        "the identity 0x61 leaves keeps the reconciled payout key"
    );
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x62, 0x72),
        [(vec![0x62; 20], true), (new_payout.to_vec(), false)]
    );
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x65, 0x65),
        [(vec![0x65; 20], false)]
    );
}

/// A round of the upgrade block that fails is thrown away with its transaction and its copy of
/// the state. The next round reconciles again, from the same committed state and Core's same
/// list, to the same result: nothing from the failed round is kept or applied twice.
#[test]
fn should_reconcile_the_same_way_when_a_round_of_the_upgrade_block_is_retried() {
    let (platform, committed, calls) = platform_after_core_history(14);
    {
        let failed_round = platform.drive.grove.start_transaction();
        run_upgrade_events(&platform, &committed, 14, &failed_round);
    }
    let transaction = platform.drive.grove.start_transaction();
    let block_state = run_upgrade_events(&platform, &committed, 14, &transaction);

    assert_eq!(
        *calls.lock().unwrap(),
        vec![(None, COMMITTED_AT), (None, COMMITTED_AT)]
    );
    let (_, _, at_committed) = core_history();
    assert_eq!(
        block_state.full_masternode_list()[&ProTxHash::from_byte_array([0x61; 32])]
            .state
            .operator_payout_address,
        at_committed[0].state.operator_payout_address
    );
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x61, 0x61),
        [
            (vec![0x61; 20], true),
            (
                at_committed[0]
                    .state
                    .operator_payout_address
                    .expect("expected Core's payout")
                    .to_vec(),
                false
            )
        ]
    );
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x62, 0x72),
        [(vec![0x62; 20], true)]
    );
}

/// The enabled TRANSFER keys of a current operator identity are not always its stored payout: up
/// to protocol version 14 a platform node id matched keys by data alone, so an evonode whose node
/// id was set to its payout's bytes and then changed again had its payout key disabled, while the
/// stored payout and Core's stayed the same. The first block of protocol version 15 reconciles
/// every current operator identity with Core's payout, so that key is enabled again.
#[test]
fn should_re_enable_a_payout_key_an_earlier_node_id_change_disabled_in_the_first_block_of_protocol_version_15(
) {
    let mut evonode = masternode(0x66, 0x66, Some(0x66));
    evonode.node_type = MasternodeType::Evo;
    evonode.state.platform_node_id = Some([0x76; 20]);
    #[allow(deprecated)]
    {
        evonode.state.legacy_platform_p2p_port = Some(26656);
        evonode.state.legacy_platform_http_port = Some(443);
    }
    let node_id_change = |node_id: [u8; 20]| MasternodeListDiff {
        base_height: 0,
        block_height: 0,
        added_mns: vec![],
        removed_mns: vec![],
        updated_mns: vec![(
            evonode.pro_tx_hash,
            diff_entry(serde_json::json!({ "platformNodeID": hex::encode(node_id) })),
        )],
    };
    let mut at_committed = evonode.clone();
    at_committed.state.platform_node_id = Some([0x86; 20]);
    let (platform, committed, _calls) = platform_after_history(
        14,
        vec![evonode.clone()],
        vec![node_id_change([0x66; 20]), node_id_change([0x86; 20])],
        vec![at_committed],
    );
    let transaction = platform.drive.grove.start_transaction();
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x66, 0x66),
        [(vec![0x66; 20], true)],
        "PV14 disabled the payout key when the node id left the payout's bytes"
    );

    run_upgrade_events(&platform, &committed, 14, &transaction);

    assert_eq!(
        transfer_keys(&platform, &transaction, 0x66, 0x66),
        [(vec![0x66; 20], false)]
    );
}

/// One operator running several masternodes usually gives them one payout address, which
/// protocol version 14 never took from Core's diffs. The first block of protocol version 15 then
/// adds that address's key to each of their operator identities in one batch, which creates the
/// address's hash tree once, also where Drive checks batches for consistency.
#[test]
fn should_reconcile_masternodes_that_share_a_new_payout_in_one_batch() {
    for batching_consistency_verification in [false, true] {
        let listed = vec![masternode(0x67, 0x67, None), masternode(0x68, 0x68, None)];
        let shared_payout_diff = MasternodeListDiff {
            base_height: 0,
            block_height: 0,
            added_mns: vec![],
            removed_mns: vec![],
            updated_mns: listed
                .iter()
                .map(|masternode| {
                    (
                        masternode.pro_tx_hash,
                        diff_entry(serde_json::json!({
                            "operatorPayoutAddress": "yXkZa7p2ne58x2XQYdNthAxfXy9cHiWxF5",
                        })),
                    )
                })
                .collect(),
        };
        let shared_payout = shared_payout_diff.updated_mns[0]
            .1
            .operator_payout_address
            .expect("expected the diff to set the payout");
        let at_committed = listed
            .iter()
            .cloned()
            .map(|mut masternode| {
                masternode.state.operator_payout_address = shared_payout;
                masternode
            })
            .collect();
        let mut config = PlatformConfig::default();
        config.drive.batching_consistency_verification = batching_consistency_verification;
        let (platform, committed, _calls) = platform_with_config_after_history(
            config,
            14,
            listed,
            vec![shared_payout_diff],
            at_committed,
            upgrade_block_diff(),
        );
        let transaction = platform.drive.grove.start_transaction();

        run_upgrade_events(&platform, &committed, 14, &transaction);

        let shared_payout = shared_payout.expect("expected a shared payout").to_vec();
        for id in [0x67, 0x68] {
            assert_eq!(
                transfer_keys(&platform, &transaction, id, id),
                [(shared_payout.clone(), false)],
                "masternode {id:#x}, consistency checks {batching_consistency_verification}"
            );
        }
    }
}

/// A diff whose entries each change a masternode's operator key, the masternode named by the
/// entry's first byte to `[key; 48]`, printed without a payout as Core prints a key change.
fn operator_key_changes(changes: &[(u8, u8)]) -> MasternodeListDiff {
    MasternodeListDiff {
        base_height: 0,
        block_height: 0,
        added_mns: vec![],
        removed_mns: vec![],
        updated_mns: changes
            .iter()
            .map(|(id, key)| {
                (
                    ProTxHash::from_byte_array([*id; 32]),
                    diff_entry(
                        serde_json::json!({ "pubKeyOperator": format!("{key:02x}").repeat(48) }),
                    ),
                )
            })
            .collect(),
    }
}

/// The credits, 0.5 Dash, the returning operator's identity holds.
const ACCRUED: Credits = 50_000_000_000;

/// Runs the first block of protocol version 15, and then its own diff, after a history at 14 in
/// which the owner of masternode `id` rotated its operator key from operator A to B and back to
/// A, each in a block of its own and without a payout, with `ACCRUED` on A's identity. Core lists
/// `core_payout` for the masternode at `COMMITTED_AT`, and the block's own diff rotates to B
/// again. Asserts that A's payout key stays its only TRANSFER key, enabled, and that a withdrawal
/// signed with it executes.
async fn assert_a_returning_operator_keeps_its_payout_key_through_the_upgrade(
    id: u8,
    core_payout: Option<[u8; 20]>,
) {
    let platform_version = PlatformVersion::latest();
    let (payout_key, payout_private_key) = IdentityPublicKey::random_key_with_known_attributes(
        0,
        &mut StdRng::seed_from_u64(id as u64),
        Purpose::TRANSFER,
        SecurityLevel::CRITICAL,
        KeyType::ECDSA_HASH160,
        None,
        platform_version,
    )
    .expect("expected a payout key");
    let payout = payout_key.data().to_vec();
    let mut listed = masternode(id, id, None);
    listed.state.operator_payout_address = Some(
        payout
            .as_slice()
            .try_into()
            .expect("expected a 20-byte payout"),
    );
    let operator_b = id ^ 0x10;
    let mut at_committed = listed.clone();
    at_committed.state.operator_payout_address = core_payout;
    let (platform, committed, _calls) = platform_with_config_after_history(
        PlatformConfig::default(),
        14,
        vec![listed.clone()],
        vec![
            operator_key_changes(&[(id, operator_b)]),
            operator_key_changes(&[(id, id)]),
        ],
        vec![at_committed],
        MasternodeListDiff {
            base_height: COMMITTED_AT,
            block_height: UPGRADE_AT,
            ..operator_key_changes(&[(id, operator_b)])
        },
    );
    let transaction = platform.drive.grove.start_transaction();
    assert_eq!(
        transfer_keys(&platform, &transaction, id, id),
        [(payout.clone(), false)],
        "PV14 kept A's payout key on A's identity"
    );
    let operator_a_identity = Identifier::create_operator_identifier(&[id; 32], &[id; 48]);
    platform
        .drive
        .add_to_system_credits(ACCRUED, Some(&transaction), platform_version)
        .expect("expected to add to system credits");
    platform
        .drive
        .add_to_identity_balance(
            operator_a_identity.to_buffer(),
            ACCRUED,
            &BlockInfo::default(),
            true,
            Some(&transaction),
            platform_version,
        )
        .expect("expected to credit A's identity");

    let mut block_state = run_upgrade_events(&platform, &committed, 14, &transaction);

    let pro_tx_hash = ProTxHash::from_byte_array([id; 32]);
    assert_eq!(
        block_state.full_masternode_list()[&pro_tx_hash]
            .state
            .operator_payout_address,
        core_payout,
        "the stored payout follows Core"
    );
    assert_eq!(
        transfer_keys(&platform, &transaction, id, id),
        [(payout.clone(), false)],
        "the upgrade left the TRANSFER keys of the identity the masternode returned to"
    );
    platform
        .update_core_info(
            Some(&committed),
            &mut block_state,
            UPGRADE_AT,
            false,
            &BlockInfo {
                height: 3,
                core_height: UPGRADE_AT,
                ..Default::default()
            },
            &transaction,
            platform_version,
        )
        .expect("expected to apply the upgrade block's diff");
    assert_eq!(
        block_state.full_masternode_list()[&pro_tx_hash]
            .state
            .pub_key_operator,
        vec![operator_b; 48]
    );
    assert_eq!(
        transfer_keys(&platform, &transaction, id, id),
        [(payout.clone(), false)],
        "A's identity keeps its payout key after the owner rotates away again"
    );
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit");

    let identity = platform
        .drive
        .fetch_full_identity(operator_a_identity.to_buffer(), None, platform_version)
        .expect("expected to fetch A's identity")
        .expect("expected A's identity");
    assert_eq!(identity.balance(), ACCRUED);
    let payout_key = identity
        .public_keys()
        .values()
        .find(|key| key.purpose() == Purpose::TRANSFER && key.data().to_vec() == payout)
        .cloned()
        .expect("expected A's payout key");
    let mut signer = SimpleSigner::default();
    signer.add_identity_public_key(payout_key.clone(), payout_private_key);
    let withdrawal = IdentityCreditWithdrawalTransition::try_from_identity(
        &identity,
        None,
        dash_to_credits!(0.1),
        Pooling::Never,
        1,
        0,
        signer,
        Some(&payout_key),
        PreferredKeyPurposeForSigningWithdrawal::TransferOnly,
        1,
        platform_version,
        None,
    )
    .await
    .expect("expected a withdrawal signed with A's payout key");
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[withdrawal
                .serialize_to_bytes()
                .expect("expected to serialize the withdrawal")],
            &block_state,
            &BlockInfo::default_with_time(1_200_001_000),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process the withdrawal");
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
}

/// Core authenticates an operator key change with the owner's key alone, so before protocol
/// version 15 an owner could rotate the operator key from operator A to B and back to A without
/// A. Core then has no payout for A, while A's identity still holds A's payout key. The first
/// block of 15 leaves the TRANSFER keys of an identity the masternode returned to as they are,
/// the rule a key change follows from 15 on: A's payout key stays enabled when the owner rotates
/// away again, and A can still withdraw the credits it accrued.
#[tokio::test]
async fn should_keep_a_returning_operators_payout_key_through_the_upgrade_when_core_reports_no_payout(
) {
    assert_a_returning_operator_keeps_its_payout_key_through_the_upgrade(0x69, None).await;
}

/// The payout Core reports for an identity a masternode returned to before protocol version 15
/// need not be its operator's own choice: the owner alone authorizes a key change. The first
/// block of 15 stores Core's payout but adds no key for it to the identity: A's payout key stays
/// enabled, and A can still withdraw the credits it accrued.
#[tokio::test]
async fn should_keep_a_returning_operators_payout_key_through_the_upgrade_when_core_reports_another_payout(
) {
    assert_a_returning_operator_keeps_its_payout_key_through_the_upgrade(0x6a, Some([0x9a; 20]))
        .await;
}

/// Only an identity a masternode returned to keeps its TRANSFER keys in the first block of
/// protocol version 15. An identity a masternode moved to and has not left is reconciled with
/// Core's payout, also after the masternode returned to an earlier identity and moved on again:
/// it loses the payout key it inherited from the previous operator up to 14, and takes Core's.
/// The identities the masternode left keep their keys.
#[test]
fn should_reconcile_an_operator_identity_the_masternode_never_left_in_the_upgrade_block() {
    let moved = masternode(0x6b, 0x6b, Some(0x6b));
    let moved_on = masternode(0x6c, 0x6c, Some(0x6c));
    let mut at_committed = vec![moved.clone(), moved_on.clone()];
    at_committed[0].state.pub_key_operator = vec![0x7b; 48];
    at_committed[0].state.operator_payout_address = Some([0x9b; 20]);
    at_committed[1].state.pub_key_operator = vec![0x8c; 48];
    at_committed[1].state.operator_payout_address = None;
    let (platform, committed, _calls) = platform_after_history(
        14,
        vec![moved.clone(), moved_on.clone()],
        vec![
            operator_key_changes(&[(0x6b, 0x7b), (0x6c, 0x7c)]),
            operator_key_changes(&[(0x6c, 0x6c)]),
            operator_key_changes(&[(0x6c, 0x8c)]),
        ],
        at_committed,
    );
    let transaction = platform.drive.grove.start_transaction();
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x6b, 0x7b),
        [(vec![0x6b; 20], false)],
        "PV14 gave the new operator the previous operator's payout key"
    );
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x6c, 0x8c),
        [(vec![0x6c; 20], false)],
        "PV14 gave the new operator the previous operator's payout key"
    );

    run_upgrade_events(&platform, &committed, 14, &transaction);

    assert_eq!(
        transfer_keys(&platform, &transaction, 0x6b, 0x7b),
        [(vec![0x6b; 20], true), (vec![0x9b; 20], false)]
    );
    assert_eq!(
        transfer_keys(&platform, &transaction, 0x6c, 0x8c),
        [(vec![0x6c; 20], true)]
    );
    for (id, key) in [(0x6b, 0x6b), (0x6c, 0x6c), (0x6c, 0x7c)] {
        assert_eq!(
            transfer_keys(&platform, &transaction, id, key),
            [(vec![id; 20], false)],
            "the identity of {key:#x} that masternode {id:#x} left keeps its keys"
        );
    }
}
