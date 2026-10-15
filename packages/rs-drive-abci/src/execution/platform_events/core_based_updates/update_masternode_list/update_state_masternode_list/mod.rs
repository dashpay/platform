mod v0;
mod v1;

// Private helpers are selected by the update_masternode_list generation.

use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeListDiff;

/// The Core diff without the operator payouts of its updated masternodes.
///
/// The generations calling this take operator payouts only from full masternode states
/// (`added_mns`), never from an updated masternode's diff: the stored list and the operator
/// identities' TRANSFER keys of every block they processed follow from that, so a diff's
/// operator payout must not reach them.
fn without_operator_payout_changes(mut masternode_diff: MasternodeListDiff) -> MasternodeListDiff {
    for (_, state_diff) in &mut masternode_diff.updated_mns {
        state_diff.operator_payout_address = None;
    }
    masternode_diff
}
