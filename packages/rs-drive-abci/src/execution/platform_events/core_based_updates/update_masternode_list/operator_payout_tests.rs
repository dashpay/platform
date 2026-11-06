//! Operator payouts from Core's diffs, as `update_masternode_list` 2 and
//! `update_operator_identity` 1 apply them from protocol version 15.

use super::test::{empty_state_diff, masternode_with_operator_payout, operator_identity_keys};
use crate::config::PlatformConfig;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc::json::{
    DMNStateDiff, MasternodeListDiff, MasternodeListItem, MasternodeType,
};
use dpp::identity::Purpose;
use dpp::util::hash::ripemd160_sha256;
use dpp::version::PlatformVersion;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

/// Core height of the full list the masternodes start from; diff `i` ends at the next heights.
const LIST_HEIGHT: u32 = 1000;

/// Processes a mock Core's full list of `masternodes` at Core height 1000 and then each of
/// `diffs`, one Core height and one block each, on a platform at `platform_version`. Returns the
/// platform, with the updates committed, and the platform state after the last diff, whose
/// change trackers hold only what that diff changed. Asked for a masternode's state, Core
/// reports no operator payout, as after an operator key change that set none.
fn masternode_list_after_diffs(
    platform_version: &PlatformVersion,
    masternodes: Vec<MasternodeListItem>,
    diffs: Vec<DMNStateDiffs>,
) -> (TempPlatform<MockCoreRPCLike>, PlatformState) {
    let (platform, state, _) =
        masternode_list_after_diffs_with_core_payouts(platform_version, masternodes, diffs, &[]);
    (platform, state)
}

/// The masternode states Platform asked Core for, as (masternode, Core height).
type StateReads = Arc<Mutex<Vec<(ProTxHash, u32)>>>;

/// `masternode_list_after_diffs`, with Core reporting `core_payouts` as the operator payout of
/// a masternode at a Core height, and no payout for any other. Also returns the masternode states
/// Platform asked Core for.
fn masternode_list_after_diffs_with_core_payouts(
    platform_version: &PlatformVersion,
    masternodes: Vec<MasternodeListItem>,
    diffs: Vec<DMNStateDiffs>,
    core_payouts: &[(ProTxHash, u32, Option<[u8; 20]>)],
) -> (TempPlatform<MockCoreRPCLike>, PlatformState, StateReads) {
    let mut platform = TestPlatformBuilder::new()
        .with_config(PlatformConfig::default())
        .with_initial_protocol_version(platform_version.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    let full_list = MasternodeListDiff {
        base_height: 0,
        block_height: LIST_HEIGHT,
        added_mns: masternodes,
        removed_mns: vec![],
        updated_mns: vec![],
    };
    let diffs = diffs
        .into_iter()
        .zip(LIST_HEIGHT..)
        .map(|(updated_mns, base_height)| MasternodeListDiff {
            base_height,
            block_height: base_height + 1,
            added_mns: vec![],
            removed_mns: vec![],
            updated_mns,
        })
        .collect::<Vec<_>>();
    let core_heights = diffs
        .iter()
        .map(|diff| diff.block_height)
        .collect::<Vec<_>>();
    platform
        .core_rpc
        .expect_get_protx_diff_with_masternodes()
        .returning(move |_base_block, block| {
            Ok(if block == LIST_HEIGHT {
                full_list.clone()
            } else {
                diffs[(block - LIST_HEIGHT - 1) as usize].clone()
            })
        });

    let core_payouts = core_payouts.to_vec();
    let reads = StateReads::default();
    let recorded_reads = Arc::clone(&reads);
    platform
        .core_rpc
        .expect_get_masternode_state()
        .returning(move |pro_tx_hash, core_height| {
            recorded_reads
                .lock()
                .expect("expected the state reads")
                .push((*pro_tx_hash, core_height));
            let mut state = masternode_with_operator_payout(0).state;
            state.operator_payout_address = core_payouts
                .iter()
                .find(|(masternode, height, _)| masternode == pro_tx_hash && *height == core_height)
                .and_then(|(_, _, payout)| *payout);
            Ok(state)
        });

    let transaction = platform.drive.grove.start_transaction();
    let mut committed: Option<PlatformState> = None;
    let mut block_state = platform.state.load().as_ref().clone();
    for (height, core_height) in (1..).zip([LIST_HEIGHT].into_iter().chain(core_heights)) {
        block_state.mark_saved();
        platform
            .update_masternode_list(
                committed.as_ref(),
                &mut block_state,
                core_height,
                committed.is_none(),
                &BlockInfo {
                    height,
                    core_height,
                    time_ms: height,
                    ..Default::default()
                },
                &transaction,
                platform_version,
            )
            .expect("expected to process Core's masternode list");
        committed = Some(block_state.clone());
    }
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit");
    (platform, block_state, reads)
}

/// The updated masternodes of one Core diff.
type DMNStateDiffs = Vec<(ProTxHash, DMNStateDiff)>;

/// The (data, disabled) of every TRANSFER key of the operator identity of `masternode` with
/// operator key `pub_key_operator`, by key id.
fn transfer_keys(
    platform: &TempPlatform<MockCoreRPCLike>,
    masternode: &MasternodeListItem,
    pub_key_operator: &[u8],
) -> Vec<(Vec<u8>, bool)> {
    operator_identity_keys(
        platform,
        None,
        &masternode.pro_tx_hash,
        pub_key_operator,
        PlatformVersion::latest(),
    )
    .into_values()
    .filter(|(purpose, _, _)| *purpose == Purpose::TRANSFER)
    .map(|(_, data, disabled)| (data, disabled))
    .collect()
}

fn stored_operator_payout(
    state: &PlatformState,
    masternode: &MasternodeListItem,
) -> Option<[u8; 20]> {
    state.full_masternode_list()[&masternode.pro_tx_hash]
        .state
        .operator_payout_address
}

/// An operator payout Core reports in a diff reaches the stored list and the operator identity:
/// the previous payout's key is disabled and the new payout's key added, also when the identity
/// had no payout key, as every masternode registered with an operator reward starts (a ProRegTx
/// carries no operator payout). The block upserts exactly those masternodes.
#[test]
fn should_add_the_new_operator_payout_key_to_the_operator_identity() {
    let changed = masternode_with_operator_payout(0x61);
    let mut first_set = masternode_with_operator_payout(0x62);
    first_set.state.operator_payout_address = None;
    let (platform, state) = masternode_list_after_diffs(
        PlatformVersion::latest(),
        vec![changed.clone(), first_set.clone()],
        vec![vec![
            (
                changed.pro_tx_hash,
                DMNStateDiff {
                    operator_payout_address: Some(Some([0x81; 20])),
                    ..empty_state_diff()
                },
            ),
            (
                first_set.pro_tx_hash,
                DMNStateDiff {
                    operator_payout_address: Some(Some([0x82; 20])),
                    ..empty_state_diff()
                },
            ),
        ]],
    );

    assert_eq!(
        transfer_keys(&platform, &changed, &changed.state.pub_key_operator),
        [(vec![0x61; 20], true), (vec![0x81; 20], false)]
    );
    assert_eq!(
        transfer_keys(&platform, &first_set, &first_set.state.pub_key_operator),
        [(vec![0x82; 20], false)]
    );
    assert_eq!(stored_operator_payout(&state, &changed), Some([0x81; 20]));
    assert_eq!(stored_operator_payout(&state, &first_set), Some([0x82; 20]));
    assert_eq!(
        state.masternode_changes.upserted,
        BTreeSet::from([changed.pro_tx_hash, first_set.pro_tx_hash])
    );
}

/// Core clears the operator payout whenever the operator key changes and prints no field for the
/// clear, so a new operator identity gets only the payout Core reports with the new key, never
/// the stored one, which is the previous operator's. The previous operator's identity keeps its
/// payout key, so its operator can still withdraw what was sent to it.
#[test]
fn should_not_give_a_new_operator_identity_the_previous_operators_payout_key() {
    let reset = masternode_with_operator_payout(0x63);
    let reset_and_set = masternode_with_operator_payout(0x64);
    let (platform, state) = masternode_list_after_diffs(
        PlatformVersion::latest(),
        vec![reset.clone(), reset_and_set.clone()],
        vec![vec![
            (
                reset.pro_tx_hash,
                DMNStateDiff {
                    pub_key_operator: Some(vec![0x73; 48]),
                    ..empty_state_diff()
                },
            ),
            (
                reset_and_set.pro_tx_hash,
                DMNStateDiff {
                    pub_key_operator: Some(vec![0x74; 48]),
                    operator_payout_address: Some(Some([0x84; 20])),
                    ..empty_state_diff()
                },
            ),
        ]],
    );

    assert_eq!(transfer_keys(&platform, &reset, &[0x73; 48]), []);
    assert_eq!(stored_operator_payout(&state, &reset), None);
    assert_eq!(
        transfer_keys(&platform, &reset_and_set, &[0x74; 48]),
        [(vec![0x84; 20], false)]
    );
    assert_eq!(
        stored_operator_payout(&state, &reset_and_set),
        Some([0x84; 20])
    );
    for masternode in [&reset, &reset_and_set] {
        assert_eq!(
            transfer_keys(&platform, masternode, &masternode.state.pub_key_operator),
            [(
                masternode.state.operator_payout_address.unwrap().to_vec(),
                false
            )],
            "the previous operator's identity of {}",
            masternode.pro_tx_hash
        );
    }
}

/// An operator key that comes back lands on the identity its earlier operator left, which still
/// holds that operator's payout key. Its TRANSFER keys are reconciled with the payout Core
/// reports now: the earlier operator's key is disabled, with or without a new payout.
#[test]
fn should_reconcile_the_payout_keys_of_an_operator_identity_a_masternode_switches_back_to() {
    let cleared = masternode_with_operator_payout(0x65);
    let replaced = masternode_with_operator_payout(0x66);
    let key_change = |masternode: &MasternodeListItem, key: u8, payout: Option<u8>| {
        (
            masternode.pro_tx_hash,
            DMNStateDiff {
                pub_key_operator: Some(vec![key; 48]),
                operator_payout_address: payout.map(|payout| Some([payout; 20])),
                ..empty_state_diff()
            },
        )
    };
    let (platform, state) = masternode_list_after_diffs(
        PlatformVersion::latest(),
        vec![cleared.clone(), replaced.clone()],
        vec![
            vec![
                key_change(&cleared, 0x75, Some(0x85)),
                key_change(&replaced, 0x76, Some(0x86)),
            ],
            vec![
                key_change(&cleared, 0x65, None),
                key_change(&replaced, 0x66, Some(0x96)),
            ],
        ],
    );

    assert_eq!(
        transfer_keys(&platform, &cleared, &cleared.state.pub_key_operator),
        [(vec![0x65; 20], true)]
    );
    assert_eq!(stored_operator_payout(&state, &cleared), None);
    assert_eq!(
        transfer_keys(&platform, &replaced, &replaced.state.pub_key_operator),
        [(vec![0x66; 20], true), (vec![0x96; 20], false)]
    );
    assert_eq!(stored_operator_payout(&state, &replaced), Some([0x96; 20]));
    // The intermediate operators keep their payout keys.
    assert_eq!(
        transfer_keys(&platform, &cleared, &[0x75; 48]),
        [(vec![0x85; 20], false)]
    );
    assert_eq!(
        transfer_keys(&platform, &replaced, &[0x76; 48]),
        [(vec![0x86; 20], false)]
    );
}

/// Core prints the stored payout again when the stored one went stale through a clear or a reset
/// Core does not print. The identity already holds that payout's key, so the update leaves it as
/// it is: adding a second key with the same data would fail on every node and halt the chain.
/// A payout that comes back after another one re-enables its key.
#[test]
fn should_not_halt_when_a_diff_re_sends_the_stored_operator_payout() {
    let re_sent = masternode_with_operator_payout(0x67);
    let back = masternode_with_operator_payout(0x68);
    let payout = |masternode: &MasternodeListItem, payout: u8| {
        (
            masternode.pro_tx_hash,
            DMNStateDiff {
                operator_payout_address: Some(Some([payout; 20])),
                ..empty_state_diff()
            },
        )
    };
    let (platform, state) = masternode_list_after_diffs(
        PlatformVersion::latest(),
        vec![re_sent.clone(), back.clone()],
        vec![
            vec![payout(&re_sent, 0x67), payout(&back, 0x88)],
            vec![payout(&re_sent, 0x67), payout(&back, 0x68)],
        ],
    );

    assert_eq!(
        transfer_keys(&platform, &re_sent, &re_sent.state.pub_key_operator),
        [(vec![0x67; 20], false)]
    );
    assert_eq!(stored_operator_payout(&state, &re_sent), Some([0x67; 20]));
    assert_eq!(
        transfer_keys(&platform, &back, &back.state.pub_key_operator),
        [(vec![0x68; 20], false), (vec![0x88; 20], true)]
    );
    assert_eq!(stored_operator_payout(&state, &back), Some([0x68; 20]));
}

/// Core prints `PoSeRevivedHeight` only in a diff that spans the revive. The stored list keeps
/// the revived height through later diffs that do not print it, in the full list and in the
/// evonode list.
#[test]
fn should_keep_the_revived_height_a_diff_does_not_print() {
    let mut revived = masternode_with_operator_payout(0x69);
    revived.node_type = MasternodeType::Evo;
    revived.state.platform_node_id = Some([0x69; 20]);
    #[allow(deprecated)]
    {
        revived.state.legacy_platform_p2p_port = Some(26656);
        revived.state.legacy_platform_http_port = Some(443);
    }
    revived.state.pose_revived_height = Some(900);
    let (_platform, state) = masternode_list_after_diffs(
        PlatformVersion::latest(),
        vec![revived.clone()],
        vec![vec![(
            revived.pro_tx_hash,
            DMNStateDiff {
                last_paid_height: Some(1001),
                ..empty_state_diff()
            },
        )]],
    );

    for list in [state.full_masternode_list(), state.hpmn_masternode_list()] {
        assert_eq!(
            list[&revived.pro_tx_hash].state.pose_revived_height,
            Some(900)
        );
    }
}

/// An evonode `id` with platform node id `[id; 20]`.
fn evonode(id: u8) -> MasternodeListItem {
    let mut evonode = masternode_with_operator_payout(id);
    evonode.node_type = MasternodeType::Evo;
    evonode.state.platform_node_id = Some([id ^ 0x0f; 20]);
    #[allow(deprecated)]
    {
        evonode.state.legacy_platform_p2p_port = Some(26656);
        evonode.state.legacy_platform_http_port = Some(443);
    }
    evonode
}

/// The (data, disabled) of every key of the operator identity of `masternode` with operator key
/// `pub_key_operator` whose purpose is `purpose`, by key id.
fn keys_with_purpose(
    platform: &TempPlatform<MockCoreRPCLike>,
    masternode: &MasternodeListItem,
    pub_key_operator: &[u8],
    purpose: Purpose,
) -> Vec<(Vec<u8>, bool)> {
    operator_identity_keys(
        platform,
        None,
        &masternode.pro_tx_hash,
        pub_key_operator,
        PlatformVersion::latest(),
    )
    .into_values()
    .filter(|(key_purpose, _, _)| *key_purpose == purpose)
    .map(|(_, data, disabled)| (data, disabled))
    .collect()
}

/// An operator payout and a platform node id are any 20 bytes Core accepts, so an operator can
/// make the payout equal the platform node id or the hash of the operator key, make the node id
/// equal the payout, or set as payout a hash that was the node id before. Each such key is added
/// and holds the authority of its kind: the payout's TRANSFER key is enabled, so the operator can
/// still withdraw, and the block does not fail on the identity already holding a key of that
/// hash. A later change of both the node id and the payout goes through.
#[test]
fn should_add_an_operator_payout_or_node_id_key_that_repeats_another_keys_hash() {
    let payout_is_node_id = evonode(0x6a);
    let payout_is_operator_key_hash = evonode(0x6b);
    let node_id_is_payout = evonode(0x6c);
    let payout_was_node_id = evonode(0x6f);
    let node_id = |masternode: &MasternodeListItem| {
        masternode
            .state
            .platform_node_id
            .expect("expected an evonode")
    };
    let payout = |masternode: &MasternodeListItem| {
        masternode
            .state
            .operator_payout_address
            .expect("expected an operator payout")
    };
    let operator_key_hash = ripemd160_sha256(&payout_is_operator_key_hash.state.pub_key_operator);
    let earlier_node_id = [0x9f; 20];
    let entry =
        |masternode: &MasternodeListItem, diff: DMNStateDiff| (masternode.pro_tx_hash, diff);
    let (platform, state) = masternode_list_after_diffs(
        PlatformVersion::latest(),
        vec![
            payout_is_node_id.clone(),
            payout_is_operator_key_hash.clone(),
            node_id_is_payout.clone(),
            payout_was_node_id.clone(),
        ],
        vec![
            vec![
                entry(
                    &payout_is_node_id,
                    DMNStateDiff {
                        operator_payout_address: Some(Some(node_id(&payout_is_node_id))),
                        ..empty_state_diff()
                    },
                ),
                entry(
                    &payout_is_operator_key_hash,
                    DMNStateDiff {
                        operator_payout_address: Some(Some(operator_key_hash)),
                        ..empty_state_diff()
                    },
                ),
                entry(
                    &node_id_is_payout,
                    DMNStateDiff {
                        platform_node_id: Some(payout(&node_id_is_payout)),
                        ..empty_state_diff()
                    },
                ),
                entry(
                    &payout_was_node_id,
                    DMNStateDiff {
                        platform_node_id: Some(earlier_node_id),
                        ..empty_state_diff()
                    },
                ),
            ],
            vec![
                entry(
                    &payout_is_node_id,
                    DMNStateDiff {
                        platform_node_id: Some([0x7a; 20]),
                        operator_payout_address: Some(Some([0x8a; 20])),
                        ..empty_state_diff()
                    },
                ),
                entry(
                    &payout_was_node_id,
                    DMNStateDiff {
                        platform_node_id: Some([0x7f; 20]),
                        ..empty_state_diff()
                    },
                ),
            ],
            vec![entry(
                &payout_was_node_id,
                DMNStateDiff {
                    operator_payout_address: Some(Some(earlier_node_id)),
                    ..empty_state_diff()
                },
            )],
        ],
    );

    let key = &payout_is_node_id.state.pub_key_operator;
    assert_eq!(
        keys_with_purpose(&platform, &payout_is_node_id, key, Purpose::TRANSFER),
        [
            (payout(&payout_is_node_id).to_vec(), true),
            (node_id(&payout_is_node_id).to_vec(), true),
            (vec![0x8a; 20], false)
        ]
    );
    assert_eq!(
        keys_with_purpose(&platform, &payout_is_node_id, key, Purpose::SYSTEM),
        [
            (key.clone(), false),
            (node_id(&payout_is_node_id).to_vec(), true),
            (vec![0x7a; 20], false)
        ]
    );
    assert_eq!(
        stored_operator_payout(&state, &payout_is_node_id),
        Some([0x8a; 20])
    );

    let key = &payout_is_operator_key_hash.state.pub_key_operator;
    assert_eq!(
        keys_with_purpose(
            &platform,
            &payout_is_operator_key_hash,
            key,
            Purpose::TRANSFER
        ),
        [
            (payout(&payout_is_operator_key_hash).to_vec(), true),
            (operator_key_hash.to_vec(), false)
        ]
    );

    let key = &node_id_is_payout.state.pub_key_operator;
    assert_eq!(
        keys_with_purpose(&platform, &node_id_is_payout, key, Purpose::TRANSFER),
        [(payout(&node_id_is_payout).to_vec(), false)]
    );
    assert_eq!(
        keys_with_purpose(&platform, &node_id_is_payout, key, Purpose::SYSTEM),
        [
            (key.clone(), false),
            (node_id(&node_id_is_payout).to_vec(), true),
            (payout(&node_id_is_payout).to_vec(), false)
        ]
    );

    let key = &payout_was_node_id.state.pub_key_operator;
    assert_eq!(
        keys_with_purpose(&platform, &payout_was_node_id, key, Purpose::TRANSFER),
        [
            (payout(&payout_was_node_id).to_vec(), true),
            (earlier_node_id.to_vec(), false)
        ]
    );
    assert_eq!(
        stored_operator_payout(&state, &payout_was_node_id),
        Some(earlier_node_id)
    );
}

/// A new operator whose payout Core reports equal to the platform node id gets an identity with
/// both keys: the payout's TRANSFER key and the platform node id key.
#[test]
fn should_create_a_new_operator_identity_whose_payout_equals_the_platform_node_id() {
    let masternode = evonode(0x6d);
    let node_id = masternode
        .state
        .platform_node_id
        .expect("expected an evonode");
    let (platform, _state) = masternode_list_after_diffs(
        PlatformVersion::latest(),
        vec![masternode.clone()],
        vec![vec![(
            masternode.pro_tx_hash,
            DMNStateDiff {
                pub_key_operator: Some(vec![0x7d; 48]),
                operator_payout_address: Some(Some(node_id)),
                ..empty_state_diff()
            },
        )]],
    );

    assert_eq!(
        keys_with_purpose(&platform, &masternode, &[0x7d; 48], Purpose::TRANSFER),
        [(node_id.to_vec(), false)]
    );
    assert_eq!(
        keys_with_purpose(&platform, &masternode, &[0x7d; 48], Purpose::SYSTEM),
        [(vec![0x7d; 48], false), (node_id.to_vec(), false)]
    );
}

/// Core reports a masternode as updated on nearly every block for its payment fields alone; such
/// a diff leaves the stored list and the platform state clean, so the block does not rewrite the
/// saved state.
#[test]
fn should_leave_the_state_clean_after_a_payment_only_diff() {
    let masternode = masternode_with_operator_payout(0x6e);
    let (_platform, state) = masternode_list_after_diffs(
        PlatformVersion::latest(),
        vec![masternode.clone()],
        vec![vec![(
            masternode.pro_tx_hash,
            DMNStateDiff {
                last_paid_height: Some(1001),
                consecutive_payments: Some(1),
                ..empty_state_diff()
            },
        )]],
    );

    assert!(!state.heavy_fields_dirty);
    assert!(state.masternode_changes.upserted.is_empty());
}

/// Core empties the operator payout on every operator key change and prints nothing for it, so
/// a key change printed without a payout does not tell whether the new operator set a payout
/// again inside the same Core heights, possibly the very one the masternode had. Platform takes
/// the payout from Core's state of that masternode at the block's Core height instead: a new
/// operator identity gets the payout Core reports, an identity a masternode returns to keeps the
/// key of the payout Core reports, and a reset leaves no payout. Core is asked only about such
/// entries.
#[test]
fn should_take_the_payout_core_reports_for_an_operator_key_change_printed_without_one() {
    let restored_on_new = masternode_with_operator_payout(0x71);
    let restored_on_returning = masternode_with_operator_payout(0x72);
    let reset = masternode_with_operator_payout(0x73);
    let key_change = |masternode: &MasternodeListItem, key: u8, payout: Option<u8>| {
        (
            masternode.pro_tx_hash,
            DMNStateDiff {
                pub_key_operator: Some(vec![key; 48]),
                operator_payout_address: payout.map(|payout| Some([payout; 20])),
                ..empty_state_diff()
            },
        )
    };
    let (platform, state, reads) = masternode_list_after_diffs_with_core_payouts(
        PlatformVersion::latest(),
        vec![
            restored_on_new.clone(),
            restored_on_returning.clone(),
            reset.clone(),
        ],
        vec![
            vec![
                key_change(&restored_on_new, 0x81, None),
                key_change(&restored_on_returning, 0x82, Some(0x92)),
                key_change(&reset, 0x83, None),
            ],
            vec![key_change(&restored_on_returning, 0x72, None)],
        ],
        &[
            (restored_on_new.pro_tx_hash, 1001, Some([0x71; 20])),
            (restored_on_returning.pro_tx_hash, 1002, Some([0x72; 20])),
        ],
    );

    assert_eq!(
        transfer_keys(&platform, &restored_on_new, &[0x81; 48]),
        [(vec![0x71; 20], false)]
    );
    assert_eq!(
        stored_operator_payout(&state, &restored_on_new),
        Some([0x71; 20])
    );
    assert_eq!(
        transfer_keys(&platform, &restored_on_returning, &[0x72; 48]),
        [(vec![0x72; 20], false)]
    );
    assert_eq!(
        stored_operator_payout(&state, &restored_on_returning),
        Some([0x72; 20])
    );
    assert_eq!(transfer_keys(&platform, &reset, &[0x83; 48]), []);
    assert_eq!(stored_operator_payout(&state, &reset), None);
    assert_eq!(
        *reads.lock().unwrap(),
        vec![
            (restored_on_new.pro_tx_hash, 1001),
            (reset.pro_tx_hash, 1001),
            (restored_on_returning.pro_tx_hash, 1002),
        ]
    );
}
