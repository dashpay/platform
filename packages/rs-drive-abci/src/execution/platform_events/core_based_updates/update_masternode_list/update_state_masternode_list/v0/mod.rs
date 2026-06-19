use crate::error::Error;
use crate::execution::types::update_state_masternode_list_outcome;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;

use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::platform_types::validator_set::ValidatorSet;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::dashcore_rpc::dashcore_rpc_json::{
    DMNState, DMNStateDiff, MasternodeListDiff, MasternodeType,
};
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
                    // The updated HPMN state is the source of truth for whether this
                    // node is still a valid platform validator: both platform ports
                    // are mandatory (`new_validator_if_masternode_in_state` rejects a
                    // node missing either). Computed before the validator-set borrow
                    // so the `hpmn_list_item` borrow ends first.
                    let resolves_platform_ports =
                        resolves_platform_validator_ports(&hpmn_list_item.state);
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
                        if resolves_platform_ports {
                            // Update the ban status / IP / platform port on the cached
                            // validator entry.
                            Self::update_masternode_in_validator_sets(
                                pro_tx_hash,
                                state_diff,
                                state.validator_sets_mut(),
                            );
                        } else {
                            // Platform ports disappeared (Core 23 `addresses` cleared,
                            // or a zeroed legacy port with no addresses) → the node is
                            // no longer a valid HPMN validator. Drop the stale cached
                            // entry so we stop advertising a dead platform endpoint.
                            Self::remove_masternode_in_validator_sets(
                                pro_tx_hash,
                                state.validator_sets_mut(),
                            );
                        }
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
///
/// The legacy fallback drops `0`: Core 23 (ProTx v3) entries zero the deprecated flat
/// port (the real port lives in `addresses`), and the nested-address accessor already
/// drops zero, so surfacing a legacy `0` here would set a validator's platform port to
/// `0` — the exact failure [rust-dashcore#808] fixed.
#[allow(deprecated)]
fn diff_platform_p2p_port(diff: &DMNStateDiff) -> Option<u32> {
    diff.platform_p2p_address()
        .map(|(_host, port)| port)
        .or_else(|| diff.legacy_platform_p2p_port.filter(|&port| port != 0))
}

/// Resolve a masternode diff's platform **HTTPS** port change — the http analogue of
/// [`diff_platform_p2p_port`] (same Core-23-nested-first, non-zero-legacy-fallback rule).
#[allow(deprecated)]
fn diff_platform_http_port(diff: &DMNStateDiff) -> Option<u32> {
    diff.platform_http_address()
        .map(|(_host, port)| port)
        .or_else(|| diff.legacy_platform_http_port.filter(|&port| port != 0))
}

/// Whether a masternode's (post-`apply_diff`) state still resolves **both** platform
/// ports — the validity condition `new_validator_if_masternode_in_state` enforces (it
/// rejects a node missing either). Uses the full-state accessors, which prefer the
/// Core 23 nested addresses and fall back to the non-zero legacy port paired with the
/// node's service IP. When this is false the node can no longer be a platform
/// validator, so its cached entry must be dropped rather than left stale.
fn resolves_platform_validator_ports(state: &DMNState) -> bool {
    state.platform_p2p_address().is_some() && state.platform_http_address().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeAddresses;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

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

    // The legacy flat port is zeroed for Core 23 (v3) entries; the resolver must drop
    // it rather than surface a validator port of 0 (rust-dashcore#808).
    #[test]
    #[allow(deprecated)]
    fn diff_drops_zero_legacy_port() {
        let diff = DMNStateDiff {
            legacy_platform_p2p_port: Some(0),
            legacy_platform_http_port: Some(0),
            ..empty_diff()
        };
        assert_eq!(diff_platform_p2p_port(&diff), None);
        assert_eq!(diff_platform_http_port(&diff), None);
    }

    #[allow(deprecated)]
    fn base_dmn_state() -> DMNState {
        DMNState {
            service: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2)), 19999),
            registered_height: 1,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: 0,
            owner_address: [0u8; 20],
            voting_address: [0u8; 20],
            payout_address: [0u8; 20],
            pub_key_operator: vec![1u8; 48],
            operator_payout_address: None,
            platform_node_id: Some([7u8; 20]),
            legacy_platform_p2p_port: None,
            legacy_platform_http_port: None,
            addresses: None,
        }
    }

    // A Core 23 node resolves both ports from `addresses` even though the legacy flat
    // fields are zeroed → still a valid platform validator.
    #[test]
    #[allow(deprecated)]
    fn resolves_ports_for_core23_addresses() {
        let state = DMNState {
            legacy_platform_p2p_port: Some(0),
            legacy_platform_http_port: Some(0),
            addresses: Some(MasternodeAddresses {
                core_p2p: vec![],
                platform_p2p: vec!["192.0.2.2:36656".to_string()],
                platform_https: vec!["192.0.2.2:443".to_string()],
            }),
            ..base_dmn_state()
        };
        assert!(resolves_platform_validator_ports(&state));
    }

    // A Core 22 node resolves both ports from the non-zero legacy fields.
    #[test]
    #[allow(deprecated)]
    fn resolves_ports_for_legacy() {
        let state = DMNState {
            legacy_platform_p2p_port: Some(26656),
            legacy_platform_http_port: Some(8443),
            ..base_dmn_state()
        };
        assert!(resolves_platform_validator_ports(&state));
    }

    // Ports gone (zeroed legacy + empty addresses, or only one port present) → no
    // longer a valid platform validator, so the cached entry must be dropped.
    #[test]
    #[allow(deprecated)]
    fn does_not_resolve_when_ports_disappear() {
        let cleared = DMNState {
            legacy_platform_p2p_port: Some(0),
            legacy_platform_http_port: Some(0),
            addresses: Some(MasternodeAddresses {
                core_p2p: vec![],
                platform_p2p: vec![],
                platform_https: vec![],
            }),
            ..base_dmn_state()
        };
        assert!(!resolves_platform_validator_ports(&cleared));

        let http_only = DMNState {
            addresses: Some(MasternodeAddresses {
                core_p2p: vec![],
                platform_p2p: vec![],
                platform_https: vec!["192.0.2.2:443".to_string()],
            }),
            ..base_dmn_state()
        };
        assert!(!resolves_platform_validator_ports(&http_only));
    }
}
