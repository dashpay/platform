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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_zero_heights() {
        let outcome = UpdateStateMasternodeListOutcome::default();
        assert_eq!(outcome.masternode_list_diff.base_height, 0);
        assert_eq!(outcome.masternode_list_diff.block_height, 0);
    }

    #[test]
    fn default_has_empty_lists() {
        let outcome = UpdateStateMasternodeListOutcome::default();
        assert!(outcome.masternode_list_diff.added_mns.is_empty());
        assert!(outcome.masternode_list_diff.removed_mns.is_empty());
        assert!(outcome.masternode_list_diff.updated_mns.is_empty());
        assert!(outcome.removed_masternodes.is_empty());
    }
}
