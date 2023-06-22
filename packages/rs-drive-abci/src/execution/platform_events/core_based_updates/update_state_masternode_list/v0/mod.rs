use crate::error::Error;
use crate::execution::types::update_state_masternode_list_outcome;
use crate::platform_types::platform::Platform;
use crate::platform_types::{platform_state, validator_set};
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{DMNStateDiff, MasternodeListDiff, MasternodeType};
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Remove a masternode from all validator sets based on its ProTxHash.
    ///
    /// This function iterates through all the validator sets and removes the given masternode
    /// using its ProTxHash. It modifies the validator_sets parameter in place.
    ///
    /// # Arguments
    ///
    /// * `pro_tx_hash` - A reference to the ProTxHash of the masternode to be removed.
    /// * `validator_sets` - A mutable reference to an IndexMap containing QuorumHash as key
    ///                      and ValidatorSet as value.
    ///
    fn remove_masternode_in_validator_sets(
        pro_tx_hash: &ProTxHash,
        validator_sets: &mut IndexMap<QuorumHash, validator_set::v0::ValidatorSet>,
    ) {
        validator_sets
            .iter_mut()
            .for_each(|(_quorum_hash, validator_set)| {
                validator_set.members.remove(pro_tx_hash);
            });
    }

    /// Updates a masternode in the validator sets.
    ///
    /// This function updates the properties of the masternode that matches the given `pro_tx_hash`.
    /// The properties are updated based on the provided `dmn_state_diff` information.
    /// If a matching masternode is found, the function updates its ban status, service address,
    /// platform P2P port, and platform HTTP port accordingly.
    ///
    /// # Arguments
    ///
    /// * `pro_tx_hash` - The `ProTxHash` of the masternode to be updated
    /// * `dmn_state_diff` - The `DMNStateDiff` containing the updated masternode information
    /// * `validator_sets` - A mutable reference to the `IndexMap<QuorumHash, ValidatorSet>`
    ///                      representing the validator sets with the quorum hash as the key
    fn update_masternode_in_validator_sets(
        pro_tx_hash: &ProTxHash,
        dmn_state_diff: &DMNStateDiff,
        validator_sets: &mut IndexMap<QuorumHash, validator_set::v0::ValidatorSet>,
    ) {
        validator_sets
            .iter_mut()
            .for_each(|(_quorum_hash, validator_set)| {
                if let Some(validator) = validator_set.members.get_mut(pro_tx_hash) {
                    if let Some(maybe_ban_height) = dmn_state_diff.pose_ban_height {
                        // the ban_height was changed
                        validator.is_banned = maybe_ban_height.is_some();
                    }
                    if let Some(address) = dmn_state_diff.service {
                        validator.node_ip = address.ip().to_string();
                    }

                    if let Some(p2p_port) = dmn_state_diff.platform_p2p_port {
                        validator.platform_p2p_port = p2p_port as u16;
                    }

                    if let Some(http_port) = dmn_state_diff.platform_http_port {
                        validator.platform_http_port = http_port as u16;
                    }
                }
            });
    }

    pub(in crate::execution) fn update_state_masternode_list_v0(
        &self,
        state: &mut platform_state::v0::PlatformState,
        core_block_height: u32,
        start_from_scratch: bool,
    ) -> Result<update_state_masternode_list_outcome::v0::UpdateStateMasternodeListOutcome, Error>
    {
        let previous_core_height = if start_from_scratch {
            // baseBlock must be a chain height and not 0
            None
        } else {
            let state_core_height = state.core_height();
            if core_block_height == state_core_height {
                return Ok(update_state_masternode_list_outcome::v0::UpdateStateMasternodeListOutcome::default());
                // no need to do anything
            }
            Some(state_core_height)
        };

        let masternode_diff = self
            .core_rpc
            .get_protx_diff_with_masternodes(previous_core_height, core_block_height)?;

        let MasternodeListDiff {
            added_mns,
            removed_mns,
            updated_mns,
            ..
        } = &masternode_diff;

        //todo: clean up
        let added_hpmns = added_mns.iter().filter_map(|masternode| {
            if masternode.node_type == MasternodeType::HighPerformance {
                Some((masternode.pro_tx_hash, masternode.clone()))
            } else {
                None
            }
        });

        if start_from_scratch {
            state.hpmn_masternode_list.clear();
            state.full_masternode_list.clear();
        }

        state.hpmn_masternode_list.extend(added_hpmns.clone());

        let added_masternodes = added_mns
            .iter()
            .map(|masternode| (masternode.pro_tx_hash, masternode.clone()));

        state.full_masternode_list.extend(added_masternodes);

        updated_mns.iter().for_each(|(pro_tx_hash, state_diff)| {
            if let Some(masternode_list_item) = state.full_masternode_list.get_mut(pro_tx_hash) {
                if let Some(hpmn_list_item) = state.hpmn_masternode_list.get_mut(pro_tx_hash) {
                    hpmn_list_item.state.apply_diff(state_diff.clone());
                    // these 3 fields are the only fields that are useful for validators. If they change we need to update
                    // validator sets
                    if state_diff.pose_ban_height.is_some()
                        || state_diff.service.is_some()
                        || state_diff.platform_p2p_port.is_some()
                    {
                        // we updated the ban status the IP or the platform port, we need to update the validator in the validator list
                        Self::update_masternode_in_validator_sets(
                            pro_tx_hash,
                            state_diff,
                            &mut state.validator_sets,
                        );
                    }
                }
                masternode_list_item.state.apply_diff(state_diff.clone());
            }
        });

        removed_mns.iter().for_each(|pro_tx_hash| {
            Self::remove_masternode_in_validator_sets(pro_tx_hash, &mut state.validator_sets);
        });

        let deleted_masternodes = removed_mns.iter().copied().collect::<BTreeSet<ProTxHash>>();

        state
            .hpmn_masternode_list
            .retain(|key, _| !deleted_masternodes.contains(key));
        let mut removed_masternodes = BTreeMap::new();

        for key in deleted_masternodes {
            if let Some(value) = state.full_masternode_list.remove(&key) {
                removed_masternodes.insert(key, value);
            }
        }

        Ok(
            update_state_masternode_list_outcome::v0::UpdateStateMasternodeListOutcome {
                masternode_list_diff: masternode_diff,
                removed_masternodes,
            },
        )
    }
}
