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

    /// Apply a [`ValidatorRefresh`] (derived from the post-`apply_diff` full state) to
    /// the matching validator in every validator set. Deriving every field from the
    /// full state — rather than patching each field from the raw diff — keeps the
    /// validity decision and the written values consistent: a partial Core 23
    /// `addresses` diff overwrites the whole nested object, so the diff alone is not a
    /// reliable source for the unchanged axis.
    fn apply_validator_refresh(
        pro_tx_hash: &ProTxHash,
        refresh: &ValidatorRefresh,
        validator_sets: &mut IndexMap<QuorumHash, ValidatorSet>,
    ) {
        validator_sets
            .iter_mut()
            .for_each(|(_quorum_hash, validator_set)| {
                if let Some(validator) = validator_set.members_mut().get_mut(pro_tx_hash) {
                    validator.is_banned = refresh.is_banned;
                    validator.node_ip = refresh.node_ip.clone();
                    validator.platform_p2p_port = refresh.platform_p2p_port;
                    validator.platform_http_port = refresh.platform_http_port;
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
                    // Derive the validator's fields from the post-`apply_diff` full
                    // state — the single source of truth (the same accessors as
                    // `new_validator_if_masternode_in_state`). `Some` means the node is
                    // still a valid HPMN platform validator (both ports + a node id);
                    // `None` means it no longer is. Computed before the validator-set
                    // borrow so the `hpmn_list_item` borrow ends first.
                    let refresh = validator_refresh_from_state(&hpmn_list_item.state);
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
                        match &refresh {
                            // Still a valid platform validator → rewrite its fields
                            // from the full state.
                            Some(refresh) => Self::apply_validator_refresh(
                                pro_tx_hash,
                                refresh,
                                state.validator_sets_mut(),
                            ),
                            // Platform endpoint disappeared (Core 23 `addresses`
                            // cleared, a zeroed legacy port with no addresses, or a
                            // missing node id) → the node is no longer a valid HPMN
                            // validator. Drop the stale cached entry so we stop
                            // advertising a dead platform endpoint.
                            None => Self::remove_masternode_in_validator_sets(
                                pro_tx_hash,
                                state.validator_sets_mut(),
                            ),
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
fn diff_platform_p2p_port(diff: &DMNStateDiff) -> Option<u16> {
    diff.platform_p2p_address()
        .map(|(_host, port)| port)
        .or(diff.legacy_platform_p2p_port)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|&port| port != 0)
}

/// Resolve a masternode diff's platform **HTTPS** port change — the http analogue of
/// [`diff_platform_p2p_port`] (same Core-23-nested-first, non-zero-legacy-fallback rule).
#[allow(deprecated)]
fn diff_platform_http_port(diff: &DMNStateDiff) -> Option<u16> {
    diff.platform_http_address()
        .map(|(_host, port)| port)
        .or(diff.legacy_platform_http_port)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|&port| port != 0)
}

/// The mutable validator fields a refresh writes, derived **once** from the
/// post-`apply_diff` full state so the validity decision and the written values can
/// never disagree.
struct ValidatorRefresh {
    node_ip: String,
    platform_p2p_port: u16,
    platform_http_port: u16,
    is_banned: bool,
}

/// Derive the [`ValidatorRefresh`] for a masternode from its (post-`apply_diff`) full
/// state, or `None` if it is no longer a valid HPMN platform validator. Mirrors the
/// exact validity gate of `new_validator_if_masternode_in_state`: both platform ports
/// must resolve (Core 23 nested addresses preferred, non-zero legacy fallback paired
/// with the service IP) **and** a `platform_node_id` must be present — a non-HPMN or
/// de-platformed node has none. The advertised `node_ip` is the platform p2p host (the
/// service IP for a legacy node, the Core 23 platform host otherwise); ports go through
/// `u16::try_from` so an out-of-range value drops the node rather than truncating.
fn validator_refresh_from_state(state: &DMNState) -> Option<ValidatorRefresh> {
    let (node_ip, platform_p2p_port) = state.platform_p2p_address()?;
    let (_http_host, platform_http_port) = state.platform_http_address()?;
    state.platform_node_id?;
    Some(ValidatorRefresh {
        node_ip,
        platform_p2p_port: u16::try_from(platform_p2p_port).ok()?,
        platform_http_port: u16::try_from(platform_http_port).ok()?,
        is_banned: state.pose_ban_height.is_some(),
    })
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

    // The diff resolver drops an out-of-range legacy port instead of truncating it via
    // `as u16` (65536 would otherwise become 0 — rust-dashcore#808).
    #[test]
    #[allow(deprecated)]
    fn diff_drops_out_of_range_legacy_port() {
        let diff = DMNStateDiff {
            legacy_platform_p2p_port: Some(65536),
            ..empty_diff()
        };
        assert_eq!(diff_platform_p2p_port(&diff), None);
    }

    // A Core 23 node resolves both ports from `addresses` even though the legacy flat
    // fields are zeroed → still a valid platform validator. The advertised `node_ip` is
    // the Core 23 platform host from `addresses`, NOT the core service IP.
    #[test]
    #[allow(deprecated)]
    fn refresh_resolves_core23_addresses_with_platform_host() {
        let state = DMNState {
            legacy_platform_p2p_port: Some(0),
            legacy_platform_http_port: Some(0),
            addresses: Some(MasternodeAddresses {
                core_p2p: vec![],
                platform_p2p: vec!["203.0.113.7:36656".to_string()],
                platform_https: vec!["203.0.113.7:443".to_string()],
            }),
            ..base_dmn_state() // service IP is 192.0.2.2 — deliberately different
        };
        let refresh = validator_refresh_from_state(&state).expect("valid v3 validator");
        assert_eq!(refresh.node_ip, "203.0.113.7");
        assert_eq!(refresh.platform_p2p_port, 36656);
        assert_eq!(refresh.platform_http_port, 443);
    }

    // A Core 22 node resolves both ports from the non-zero legacy fields, paired with
    // the core service IP.
    #[test]
    #[allow(deprecated)]
    fn refresh_resolves_legacy_with_service_ip() {
        let state = DMNState {
            legacy_platform_p2p_port: Some(26656),
            legacy_platform_http_port: Some(8443),
            ..base_dmn_state()
        };
        let refresh = validator_refresh_from_state(&state).expect("valid legacy validator");
        assert_eq!(refresh.node_ip, "192.0.2.2");
        assert_eq!(refresh.platform_p2p_port, 26656);
        assert_eq!(refresh.platform_http_port, 8443);
    }

    // Ports gone (zeroed legacy + empty addresses, or only one port present) → no
    // longer a valid platform validator; the cached entry must be dropped.
    #[test]
    #[allow(deprecated)]
    fn refresh_none_when_ports_disappear() {
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
        assert!(validator_refresh_from_state(&cleared).is_none());

        let http_only = DMNState {
            addresses: Some(MasternodeAddresses {
                core_p2p: vec![],
                platform_p2p: vec![],
                platform_https: vec!["192.0.2.2:443".to_string()],
            }),
            ..base_dmn_state()
        };
        assert!(validator_refresh_from_state(&http_only).is_none());
    }

    // An HPMN that resolves both ports but has no `platform_node_id` is not a valid
    // platform validator (mirrors new_validator_if_masternode_in_state's node-id gate).
    #[test]
    #[allow(deprecated)]
    fn refresh_none_without_platform_node_id() {
        let state = DMNState {
            legacy_platform_p2p_port: Some(26656),
            legacy_platform_http_port: Some(8443),
            platform_node_id: None,
            ..base_dmn_state()
        };
        assert!(validator_refresh_from_state(&state).is_none());
    }
}
