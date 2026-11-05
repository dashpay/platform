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
use dpp::version::PlatformVersion;
use std::collections::BTreeSet;

/// Core height of the full list the masternodes start from; diff `i` ends at the next heights.
const LIST_HEIGHT: u32 = 1000;

/// Processes a mock Core's full list of `masternodes` at Core height 1000 and then each of
/// `diffs`, one Core height and one block each, on a platform at `platform_version`. Returns the
/// platform, with the updates committed, and the platform state after the last diff, whose
/// change trackers hold only what that diff changed.
fn masternode_list_after_diffs(
    platform_version: &PlatformVersion,
    masternodes: Vec<MasternodeListItem>,
    diffs: Vec<DMNStateDiffs>,
) -> (TempPlatform<MockCoreRPCLike>, PlatformState) {
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
    (platform, block_state)
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
