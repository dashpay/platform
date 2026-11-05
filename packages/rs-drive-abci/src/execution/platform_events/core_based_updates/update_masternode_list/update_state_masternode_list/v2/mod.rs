use super::is_basic_re_encoding_of_legacy_key;
use crate::error::Error;
use crate::execution::types::update_state_masternode_list_outcome;
use crate::platform_types::masternode::v0::MasternodeStateV0;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;

use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::dashcore_rpc::dashcore_rpc_json::{
    DMNState, DMNStateDiff, MasternodeListDiff, MasternodeListItem,
};
use dpp::version::PlatformVersion;
use drive::drive::system::core_fork_activations::CoreFork;
use drive::grovedb::Transaction;
use std::collections::{BTreeMap, BTreeSet};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// The quorum hashes of the validator sets the masternode is a member of.
    fn validator_sets_with_member_v2(
        state: &PlatformState,
        pro_tx_hash: &ProTxHash,
    ) -> Vec<QuorumHash> {
        state
            .validator_sets()
            .iter()
            .filter(|(_, validator_set)| validator_set.members().contains_key(pro_tx_hash))
            .map(|(quorum_hash, _)| *quorum_hash)
            .collect()
    }

    /// Remove a masternode from every validator set it is a member of.
    ///
    /// Only the validator sets that list the masternode are touched, so only
    /// their entries are rewritten by the per-entry store.
    fn remove_masternode_in_validator_sets_v2(pro_tx_hash: &ProTxHash, state: &mut PlatformState) {
        for quorum_hash in Self::validator_sets_with_member_v2(state, pro_tx_hash) {
            if let Some(validator_set) = state.validator_set_mut(&quorum_hash) {
                validator_set.members_mut().remove(pro_tx_hash);
            }
        }
    }

    /// Updates a masternode in the validator sets.
    ///
    /// This function updates the properties of the masternode that matches the given `pro_tx_hash`.
    /// The ban status and service address are updated based on the provided `dmn_state_diff`
    /// information, the platform P2P and HTTP ports from `new_platform_ports`.
    ///
    /// # Arguments
    ///
    /// * `pro_tx_hash` - The `ProTxHash` of the masternode to be updated
    /// * `dmn_state_diff` - The `DMNStateDiff` containing the updated masternode information
    /// * `new_platform_ports` - The platform ports that resolve to a new value in the updated
    ///   masternode state
    /// * `state` - The platform state whose validator sets are updated; only the
    ///   sets that list the masternode are touched
    fn update_masternode_in_validator_sets_v2(
        pro_tx_hash: &ProTxHash,
        dmn_state_diff: &DMNStateDiff,
        new_platform_ports: PlatformPorts,
        state: &mut PlatformState,
    ) {
        for quorum_hash in Self::validator_sets_with_member_v2(state, pro_tx_hash) {
            let Some(validator_set) = state.validator_set_mut(&quorum_hash) else {
                continue;
            };
            let Some(validator) = validator_set.members_mut().get_mut(pro_tx_hash) else {
                continue;
            };
            if let Some(maybe_ban_height) = dmn_state_diff.pose_ban_height {
                // the ban_height was changed
                validator.is_banned = maybe_ban_height.is_some();
            }
            if let Some(address) = dmn_state_diff.service {
                validator.node_ip = address.ip().to_string();
            }

            if let Some(p2p_port) = new_platform_ports.p2p {
                validator.platform_p2p_port = p2p_port;
            }

            if let Some(http_port) = new_platform_ports.http {
                validator.platform_http_port = http_port;
            }
        }
    }

    /// Whether applying `updated_mns` would change any item of `masternode_list`.
    ///
    /// Core reports a masternode as updated whenever any field of its state
    /// moved, including the payment and penalty fields — last paid height,
    /// consecutive payments, PoSe penalty — that change on nearly every Core
    /// block and that the stored item does not keep. Applying each diff to a
    /// copy tells the two apart.
    fn any_stored_masternode_changes_v2(
        masternode_list: &BTreeMap<ProTxHash, MasternodeListItem>,
        updated_mns: &[(ProTxHash, DMNStateDiff)],
    ) -> bool {
        updated_mns.iter().any(|(pro_tx_hash, state_diff)| {
            masternode_list.get(pro_tx_hash).is_some_and(|item| {
                apply_diff_in_stored_form(&item.state, state_diff)
                    != legacy_stored_form(item.state.clone())
            })
        })
    }

    /// Clears `pub_key_operator` in each entry of `updated_mns` whose reported key is the key
    /// `masternode_list` already holds for that masternode, once Dash Core's V24 is recorded as
    /// active: the same bytes, or the basic-scheme encoding of the key the stored bytes encode in
    /// the legacy scheme.
    ///
    /// Once V24 is active, Core re-serializes a legacy-scheme operator key in the basic scheme
    /// when it raises a masternode's state version, and reports the new bytes as a changed key.
    /// The key is the same, so the masternode keeps the operator identity derived from the
    /// stored bytes. Before V24 every entry stays as Core reports it.
    ///
    /// V24 is read from the Core fork record, at most once and only when an entry would be
    /// cleared. This runs inside `update_core_info`, after it recorded the forks active at
    /// `core_block_height` and only when that height moved, so the record answers for this very
    /// height, as Core would.
    fn drop_unchanged_operator_keys_v2(
        &self,
        masternode_list: &BTreeMap<ProTxHash, MasternodeListItem>,
        updated_mns: &mut [(ProTxHash, DMNStateDiff)],
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut v24_active = false;
        for (pro_tx_hash, state_diff) in updated_mns.iter_mut() {
            let (Some(masternode), Some(new_key)) = (
                masternode_list.get(pro_tx_hash),
                state_diff.pub_key_operator.as_deref(),
            ) else {
                continue;
            };
            let stored_key = masternode.state.pub_key_operator.as_slice();
            if new_key != stored_key && !is_basic_re_encoding_of_legacy_key(stored_key, new_key) {
                continue;
            }
            if !v24_active {
                if self
                    .drive
                    .fetch_core_fork_activation_height(
                        CoreFork::V24,
                        Some(transaction),
                        platform_version,
                    )?
                    .is_none()
                {
                    return Ok(());
                }
                v24_active = true;
            }
            state_diff.pub_key_operator = None;
        }
        Ok(())
    }

    /// Updates the masternode lists like version 1, except for operator payouts and revived
    /// heights.
    ///
    /// A diff's operator payout reaches the stored list and the identity updates. Core prints
    /// no field when it empties a payout, but it empties it on every operator key change, so an
    /// entry left with a real key change and no payout gets an explicit clear
    /// (`Some(None)`): the new operator starts without the previous operator's payout. A
    /// payout Core prints with the key change wins. A diff applies with `DMNState::apply_diff`,
    /// which keeps a stored revived height the diff does not print.
    ///
    /// An operator key Core re-encoded is the stored key, and is dropped from its entry before
    /// the clear is inferred, so a re-encode never reads as a key change.
    pub(crate) fn update_state_masternode_list_v2(
        &self,
        state: &mut PlatformState,
        core_block_height: u32,
        start_from_scratch: bool,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
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

        let mut masternode_diff = self
            .core_rpc
            .get_protx_diff_with_masternodes(previous_core_height, core_block_height)?;

        if !start_from_scratch {
            self.drop_unchanged_operator_keys_v2(
                state.full_masternode_list(),
                &mut masternode_diff.updated_mns,
                transaction,
                platform_version,
            )?;
            for (_, state_diff) in &mut masternode_diff.updated_mns {
                if state_diff.pub_key_operator.is_some()
                    && state_diff.operator_payout_address.is_none()
                {
                    state_diff.operator_payout_address = Some(None);
                }
            }
        }

        let MasternodeListDiff {
            added_mns,
            removed_mns,
            updated_mns,
            ..
        } = &masternode_diff;

        // Core advances a block without any stored masternode field changing far
        // more often than not: on mainnet almost every block's diff is only the
        // payment fields of the masternode paid in it. Returning before the
        // first mutable borrow keeps the platform state clean, which is what
        // lets the block skip rewriting the full saved state (over a megabyte
        // on mainnet) to disk.
        if !start_from_scratch
            && added_mns.is_empty()
            && removed_mns.is_empty()
            && !Self::any_stored_masternode_changes_v2(state.full_masternode_list(), updated_mns)
        {
            return Ok(
                update_state_masternode_list_outcome::v0::UpdateStateMasternodeListOutcome {
                    masternode_list_diff: masternode_diff,
                    removed_masternodes: BTreeMap::new(),
                },
            );
        }

        // Every change below goes through an accessor that records which
        // masternode or validator set it touched, so the store writes only
        // those entries.
        if start_from_scratch {
            state.clear_masternode_lists();
        }

        for masternode in added_mns {
            let mut masternode = masternode.clone();
            masternode.state = in_stored_form(masternode.state);
            state.insert_masternode(masternode);
        }

        for (pro_tx_hash, state_diff) in updated_mns {
            let Some(mut masternode) = state.full_masternode_list().get(pro_tx_hash).cloned()
            else {
                continue;
            };
            // Old in-memory addresses were not persisted before this generation. Start
            // from the stored state so a running node and a restarted node agree.
            let previous_state = legacy_stored_form(masternode.state.clone());
            let updated_state = apply_diff_in_stored_form(&previous_state, state_diff);
            let is_hpmn = state.hpmn_masternode_list().contains_key(pro_tx_hash);
            let platform_ports_before = is_hpmn.then(|| PlatformPorts::of(&previous_state));
            let platform_ports = is_hpmn.then(|| PlatformPorts::of(&updated_state));
            if updated_state != previous_state {
                masternode.state = updated_state;
                state.insert_masternode(masternode);
            }
            // Only evonodes are validators.
            let (Some(platform_ports_before), Some(platform_ports)) =
                (platform_ports_before, platform_ports)
            else {
                continue;
            };
            // The ports are read from the updated state, not from the diff: an
            // ExtAddr evonode's port move reaches us only in `addresses`, and a
            // legacy evonode's diff lists there just the entries that changed.
            let new_platform_ports = changed_platform_ports(platform_ports_before, platform_ports);
            // the ban status, the IP and the platform P2P port are the only fields that are useful for
            // validators. If they change we need to update validator sets
            if state_diff.pose_ban_height.is_some()
                || state_diff.service.is_some()
                || new_platform_ports.p2p.is_some()
            {
                // we updated the ban status the IP or the platform port, we need to update the validator in the validator list
                Self::update_masternode_in_validator_sets_v2(
                    pro_tx_hash,
                    state_diff,
                    new_platform_ports,
                    state,
                );
            }
        }

        for pro_tx_hash in removed_mns {
            Self::remove_masternode_in_validator_sets_v2(pro_tx_hash, state);
        }

        let deleted_masternodes = removed_mns.iter().copied().collect::<BTreeSet<ProTxHash>>();

        let mut removed_masternodes = BTreeMap::new();

        for key in deleted_masternodes {
            if let Some(value) = state.remove_masternode(&key) {
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

/// The ports of `after` that resolve to a value other than in `before`.
///
/// A port that stops resolving is not reported, so the validator keeps its
/// last known port.
fn changed_platform_ports(before: PlatformPorts, after: PlatformPorts) -> PlatformPorts {
    PlatformPorts {
        p2p: after.p2p.filter(|_| after.p2p != before.p2p),
        http: after.http.filter(|_| after.http != before.http),
    }
}

/// The old persisted state, without transient address or payout lists.
fn legacy_stored_form(state: DMNState) -> DMNState {
    MasternodeStateV0::from(state).into()
}

/// Resolves nested ports into the unchanged persisted fields. The global
/// converters stay literal so earlier protocol generations keep their behavior.
fn in_stored_form(mut state: DMNState) -> DMNState {
    let p2p = state.platform_p2p_address().map(|(_, port)| port);
    let http = state.platform_http_address().map(|(_, port)| port);
    #[allow(deprecated)]
    {
        state.legacy_platform_p2p_port = p2p.or(state.legacy_platform_p2p_port);
        state.legacy_platform_http_port = http.or(state.legacy_platform_http_port);
    }
    legacy_stored_form(state)
}

fn apply_diff_in_stored_form(state: &DMNState, diff: &DMNStateDiff) -> DMNState {
    let mut updated = legacy_stored_form(state.clone());
    updated.apply_diff(diff.clone());
    in_stored_form(updated)
}

/// Platform ports resolved for this generation's validator updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformPorts {
    p2p: Option<u16>,
    http: Option<u16>,
}

impl PlatformPorts {
    fn of(state: &DMNState) -> Self {
        Self {
            p2p: state
                .platform_p2p_address()
                .and_then(|(_, port)| u16::try_from(port).ok()),
            http: state
                .platform_http_address()
                .and_then(|(_, port)| u16::try_from(port).ok()),
        }
    }
}
