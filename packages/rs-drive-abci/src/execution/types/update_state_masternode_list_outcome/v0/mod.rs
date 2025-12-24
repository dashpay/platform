use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc::dashcore_rpc_json::{MasternodeListDiff, MasternodeListItem};
use std::collections::BTreeMap;

/// Represents the outcome of an attempt to update the state of a masternode list.
pub struct UpdateStateMasternodeListOutcome {
    /// The diff between two masternode lists.
    pub masternode_list_diff: MasternodeListDiff,
    /// The set of ProTxHashes that correspond to masternodes that were deleted from the list.
    pub removed_masternodes: BTreeMap<ProTxHash, MasternodeListItem>,
}

impl Default for UpdateStateMasternodeListOutcome {
    fn default() -> Self {
        UpdateStateMasternodeListOutcome {
            masternode_list_diff: MasternodeListDiff {
                base_height: 0,
                block_height: 0,
                added_mns: vec![],
                removed_mns: vec![],
                updated_mns: vec![],
            },
            removed_masternodes: Default::default(),
        }
    }
}
