use crate::error::Error;
use crate::execution::types::update_state_masternode_list_outcome;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;

use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::platform_types::validator_set::ValidatorSet;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNStateDiff, MasternodeListDiff, MasternodeType};
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
    ///   and ValidatorSet as value.
    ///
    fn remove_masternode_in_validator_sets(
        pro_tx_hash: &ProTxHash,
        validator_sets: &mut IndexMap<QuorumHash, ValidatorSet>,
    ) {
        validator_sets
            .iter_mut()
            .for_each(|(_quorum_hash, validator_set)| {
                validator_set.members_mut().remove(pro_tx_hash);
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
    ///   representing the validator sets with the quorum hash as the key
    fn update_masternode_in_validator_sets(
        pro_tx_hash: &ProTxHash,
        dmn_state_diff: &DMNStateDiff,
        validator_sets: &mut IndexMap<QuorumHash, ValidatorSet>,
    ) {
        validator_sets
            .iter_mut()
            .for_each(|(_quorum_hash, validator_set)| {
                if let Some(validator) = validator_set.members_mut().get_mut(pro_tx_hash) {
                    if let Some(maybe_ban_height) = dmn_state_diff.pose_ban_height {
                        // the ban_height was changed
                        validator.is_banned = maybe_ban_height.is_some();
                    }
                    if let Some(address) = dmn_state_diff.service {
                        validator.node_ip = address.ip().to_string();
                    }

                    if let Some(p2p_port) = diff_platform_p2p_port(dmn_state_diff) {
                        validator.platform_p2p_port = p2p_port as u16;
                    }

                    if let Some(http_port) = diff_platform_http_port(dmn_state_diff) {
                        validator.platform_http_port = http_port as u16;
                    }
                }
            });
    }

    pub(crate) fn update_state_masternode_list_v0(
        &self,
        state: &mut PlatformState,
        core_block_height: u32,
        start_from_scratch: bool,
    ) -> Result<update_state_masternode_list_outcome::v0::UpdateStateMasternodeListOutcome, Error>
    {
        let previous_core_height = if start_from_scratch {
            // baseBlock must be a chain height and not 0
            None
        } else {
            let state_core_height = state.last_committed_core_height();
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
            if masternode.node_type == MasternodeType::Evo {
                Some((masternode.pro_tx_hash, masternode.clone()))
            } else {
                None
            }
        });

        if start_from_scratch {
            state.hpmn_masternode_list_mut().clear();
            state.full_masternode_list_mut().clear();
        }

        state.hpmn_masternode_list_mut().extend(added_hpmns.clone());

        let added_masternodes = added_mns
            .iter()
            .map(|masternode| (masternode.pro_tx_hash, masternode.clone()));

        state.full_masternode_list_mut().extend(added_masternodes);

        updated_mns.iter().for_each(|(pro_tx_hash, state_diff)| {
            if let Some(masternode_list_item) =
                state.full_masternode_list_mut().get_mut(pro_tx_hash)
            {
                masternode_list_item.state.apply_diff(state_diff.clone());
                if let Some(hpmn_list_item) = state.hpmn_masternode_list_mut().get_mut(pro_tx_hash)
                {
                    hpmn_list_item.state.apply_diff(state_diff.clone());
                    // Refresh the validator entry on any change to the fields it
                    // carries: ban status, service IP, or either platform port.
                    // A platform-port change can be a resolvable p2p/http port
                    // (Core 23 nested addresses OR a legacy flat field) or an
                    // `addresses` delta that clears/empties a port — `addresses`
                    // is three-state (None / Some(None) / Some(Some)), so check
                    // its presence too, otherwise an http-only or address-clearing
                    // diff would leave a stale port on the cached validator.
                    if state_diff.pose_ban_height.is_some()
                        || state_diff.service.is_some()
                        || diff_platform_p2p_port(state_diff).is_some()
                        || diff_platform_http_port(state_diff).is_some()
                        || state_diff.addresses.is_some()
                    {
                        // we updated the ban status the IP or the platform port, we need to update the validator in the validator list
                        Self::update_masternode_in_validator_sets(
                            pro_tx_hash,
                            state_diff,
                            state.validator_sets_mut(),
                        );
                    }
                }
            }
        });

        removed_mns.iter().for_each(|pro_tx_hash| {
            Self::remove_masternode_in_validator_sets(pro_tx_hash, state.validator_sets_mut());
        });

        let deleted_masternodes = removed_mns.iter().copied().collect::<BTreeSet<ProTxHash>>();

        state
            .hpmn_masternode_list_mut()
            .retain(|key, _| !deleted_masternodes.contains(key));
        let mut removed_masternodes = BTreeMap::new();

        for key in deleted_masternodes {
            if let Some(value) = state.full_masternode_list_mut().remove(&key) {
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

/// Resolve a masternode diff's platform **P2P** port change, preferring the Core 23
/// nested `addresses` (via `DMNStateDiff::platform_p2p_address`, which — unlike
/// `DMNState`'s accessor — does NOT fall back to the legacy field) and falling back
/// to the legacy flat field. `None` when the diff carries no resolvable p2p port.
#[allow(deprecated)]
fn diff_platform_p2p_port(diff: &DMNStateDiff) -> Option<u32> {
    diff.platform_p2p_address()
        .map(|(_host, port)| port)
        .or(diff.legacy_platform_p2p_port)
}

/// Resolve a masternode diff's platform **HTTPS** port change — the http analogue of
/// [`diff_platform_p2p_port`].
#[allow(deprecated)]
fn diff_platform_http_port(diff: &DMNStateDiff) -> Option<u32> {
    diff.platform_http_address()
        .map(|(_host, port)| port)
        .or(diff.legacy_platform_http_port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeAddresses;

    #[allow(deprecated)]
    fn empty_diff() -> DMNStateDiff {
        DMNStateDiff {
            service: None,
            registered_height: None,
            last_paid_height: None,
            consecutive_payments: None,
            pose_penalty: None,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: None,
            owner_address: None,
            voting_address: None,
            payout_address: None,
            pub_key_operator: None,
            operator_payout_address: None,
            platform_node_id: None,
            legacy_platform_p2p_port: None,
            legacy_platform_http_port: None,
            addresses: None,
        }
    }

    // A Core 23 diff carries the ports in the nested `addresses` (legacy fields
    // absent); the diff accessor reads them and our resolver returns them.
    #[test]
    fn diff_resolves_core23_nested_ports() {
        let mut diff = empty_diff();
        diff.addresses = Some(Some(MasternodeAddresses {
            core_p2p: vec!["192.0.2.2:9999".to_string()],
            platform_p2p: vec!["192.0.2.2:36656".to_string()],
            platform_https: vec!["192.0.2.2:443".to_string()],
        }));
        assert_eq!(diff_platform_p2p_port(&diff), Some(36656));
        assert_eq!(diff_platform_http_port(&diff), Some(443));
    }

    // An http-only Core 23 diff (empty platform_p2p) must still resolve the http
    // port — the case the refresh-trigger guard previously missed.
    #[test]
    fn diff_resolves_http_only_core23() {
        let mut diff = empty_diff();
        diff.addresses = Some(Some(MasternodeAddresses {
            core_p2p: vec![],
            platform_p2p: vec![],
            platform_https: vec!["192.0.2.2:443".to_string()],
        }));
        assert_eq!(diff_platform_p2p_port(&diff), None);
        assert_eq!(diff_platform_http_port(&diff), Some(443));
    }

    // A Core 22 diff carries the deprecated flat ports; the resolver falls back to
    // them (the diff accessor alone returns None).
    #[test]
    #[allow(deprecated)]
    fn diff_falls_back_to_legacy_ports() {
        let diff = DMNStateDiff {
            legacy_platform_p2p_port: Some(26656),
            legacy_platform_http_port: Some(8443),
            ..empty_diff()
        };
        assert_eq!(diff_platform_p2p_port(&diff), Some(26656));
        assert_eq!(diff_platform_http_port(&diff), Some(8443));
    }

    // A diff with no platform-port change resolves to nothing on either axis.
    #[test]
    fn diff_without_port_change_is_none() {
        let diff = empty_diff();
        assert_eq!(diff_platform_p2p_port(&diff), None);
        assert_eq!(diff_platform_http_port(&diff), None);
    }
}
