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
use std::collections::{BTreeMap, BTreeSet};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// The quorum hashes of the validator sets the masternode is a member of.
    fn validator_sets_with_member_v1(
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
    fn remove_masternode_in_validator_sets_v1(pro_tx_hash: &ProTxHash, state: &mut PlatformState) {
        for quorum_hash in Self::validator_sets_with_member_v1(state, pro_tx_hash) {
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
    fn update_masternode_in_validator_sets_v1(
        pro_tx_hash: &ProTxHash,
        dmn_state_diff: &DMNStateDiff,
        new_platform_ports: PlatformPorts,
        state: &mut PlatformState,
    ) {
        for quorum_hash in Self::validator_sets_with_member_v1(state, pro_tx_hash) {
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
    fn any_stored_masternode_changes_v1(
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

    pub(crate) fn update_state_masternode_list_v1(
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

        // Core advances a block without any stored masternode field changing far
        // more often than not: on mainnet almost every block's diff is only the
        // payment fields of the masternode paid in it. Returning before the
        // first mutable borrow keeps the platform state clean, which is what
        // lets the block skip rewriting the full saved state (over a megabyte
        // on mainnet) to disk.
        if !start_from_scratch
            && added_mns.is_empty()
            && removed_mns.is_empty()
            && !Self::any_stored_masternode_changes_v1(state.full_masternode_list(), updated_mns)
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
                Self::update_masternode_in_validator_sets_v1(
                    pro_tx_hash,
                    state_diff,
                    new_platform_ports,
                    state,
                );
            }
        }

        for pro_tx_hash in removed_mns {
            Self::remove_masternode_in_validator_sets_v1(pro_tx_hash, state);
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

#[cfg(test)]
mod tests {
    use super::{in_stored_form, PlatformPorts};
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::platform_types::masternode::v0::MasternodeV0;
    use crate::platform_types::platform::Platform;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::platform_types::validator::v0::NewValidatorIfMasternodeInState;
    use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
    use crate::platform_types::validator_set::{ValidatorSet, ValidatorSetExt};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey};
    use dpp::core_types::validator::v0::ValidatorV0;
    use dpp::core_types::validator_set::v0::ValidatorSetV0;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{Network, ProTxHash, PubkeyHash, QuorumHash, ScriptBuf, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNPayout, DMNState, DMNStateDiff, MasternodeAddresses, MasternodeListDiff,
        MasternodeListItem, MasternodeType,
    };
    use dpp::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::{BTreeMap, BTreeSet};

    fn masternode(pro_tx_hash: ProTxHash) -> MasternodeListItem {
        MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array([0u8; 32]),
            collateral_index: 0,
            collateral_address: Some([0u8; 20]),
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:1234".parse().expect("socket address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: Some([0u8; 20]),
                voting_address: [0u8; 20],
                payout_address: Some([0u8; 20]),
                payouts: None,
                pub_key_operator: vec![0u8; 48],
                operator_payout_address: None,
                platform_node_id: None,
                #[allow(deprecated)]
                legacy_platform_p2p_port: None,
                #[allow(deprecated)]
                legacy_platform_http_port: None,
                addresses: None,
            },
        }
    }

    /// A diff with no field set: what Core reports for a masternode is added
    /// on top of this.
    fn empty_state_diff() -> DMNStateDiff {
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
            payouts: None,
            pub_key_operator: None,
            operator_payout_address: None,
            platform_node_id: None,
            #[allow(deprecated)]
            legacy_platform_p2p_port: None,
            #[allow(deprecated)]
            legacy_platform_http_port: None,
            addresses: None,
        }
    }

    /// A clean state holding one masternode, and a platform whose Core reports
    /// `state_diff` for it on the next block.
    fn state_with_one_masternode_and_diff(
        state_diff: DMNStateDiff,
    ) -> (TempPlatform<MockCoreRPCLike>, PlatformState, ProTxHash) {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();

        let pro_tx_hash = ProTxHash::from_byte_array([0x77u8; 32]);
        let mut state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        state.insert_masternode(masternode(pro_tx_hash));
        state.mark_saved();

        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |base_block, block| {
                Ok(MasternodeListDiff {
                    base_height: base_block.unwrap_or_default(),
                    block_height: block,
                    added_mns: vec![],
                    removed_mns: vec![],
                    updated_mns: vec![(pro_tx_hash, state_diff.clone())],
                })
            });

        (platform, state, pro_tx_hash)
    }

    /// Core reports the masternode paid in a block as updated, but the stored
    /// item keeps none of the payment fields, so nothing changes and the state
    /// must stay clean: this is what almost every mainnet block's diff is.
    #[test]
    fn payment_only_update_leaves_the_state_clean() {
        let (platform, mut state, pro_tx_hash) = state_with_one_masternode_and_diff(DMNStateDiff {
            last_paid_height: Some(2_129_183),
            consecutive_payments: Some(1),
            pose_penalty: Some(0),
            ..empty_state_diff()
        });
        let before = state.full_masternode_list().clone();

        platform
            .update_state_masternode_list_v1(&mut state, 1, false)
            .expect("update must succeed");

        assert!(
            !state.heavy_fields_dirty,
            "a diff that changes no stored field must not dirty the state"
        );
        assert!(
            state.masternode_changes.is_empty(),
            "and no entry needs rewriting"
        );
        assert_eq!(state.full_masternode_list(), &before);
        assert!(state.full_masternode_list().contains_key(&pro_tx_hash));
    }

    /// Transient nested addresses from a previous generation do not constitute
    /// a stored change when Core reports only payment metadata.
    #[test]
    fn should_keep_a_payment_only_diff_clean_with_old_transient_addresses() {
        let (platform, mut state, pro_tx_hash) = state_with_one_masternode_and_diff(DMNStateDiff {
            last_paid_height: Some(42),
            ..empty_state_diff()
        });
        let mut item = evonode();
        item.state.addresses = Some(moved_platform_ports());
        state.insert_masternode(item);
        state.mark_saved();
        let before = state.full_masternode_list()[&pro_tx_hash].clone();

        platform
            .update_state_masternode_list_v1(&mut state, 1, false)
            .expect("update");

        assert!(!state.heavy_fields_dirty);
        assert!(state.masternode_changes.is_empty());
        assert_eq!(state.full_masternode_list()[&pro_tx_hash], before);
    }

    /// A diff that does change a stored field is applied and dirties the state,
    /// so the check above does not swallow real changes.
    #[test]
    fn stored_field_update_is_applied_and_dirties_the_state() {
        let new_service = "5.6.7.8:5678".parse().expect("socket address");
        let (platform, mut state, pro_tx_hash) = state_with_one_masternode_and_diff(DMNStateDiff {
            last_paid_height: Some(2_129_183),
            service: Some(new_service),
            ..empty_state_diff()
        });

        platform
            .update_state_masternode_list_v1(&mut state, 1, false)
            .expect("update must succeed");

        assert!(state.heavy_fields_dirty);
        assert_eq!(
            state.masternode_changes.upserted,
            BTreeSet::from([pro_tx_hash]),
            "exactly the changed masternode's entry is rewritten"
        );
        assert!(!state.masternode_changes.rewrite_all);
        assert_eq!(
            state
                .full_masternode_list()
                .get(&pro_tx_hash)
                .expect("masternode stays listed")
                .state
                .service,
            new_service
        );
    }

    #[test]
    fn should_only_upsert_changed_entries_in_a_mixed_masternode_diff() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let mut state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        let changed_hash = ProTxHash::from_byte_array([0x11; 32]);
        let payment_only_hash = ProTxHash::from_byte_array([0x22; 32]);
        for pro_tx_hash in [changed_hash, payment_only_hash] {
            let mut item = evonode();
            item.pro_tx_hash = pro_tx_hash;
            // Transient addresses can survive in memory from the preceding
            // protocol generation, but do not make payment metadata a write.
            item.state.addresses = Some(moved_platform_ports());
            state.insert_masternode(item);
        }
        state.mark_saved();
        let unchanged = state.full_masternode_list()[&payment_only_hash].clone();
        let new_service = "5.6.7.8:5678".parse().expect("socket address");

        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |base_block, block| {
                Ok(MasternodeListDiff {
                    base_height: base_block.unwrap_or_default(),
                    block_height: block,
                    added_mns: vec![],
                    removed_mns: vec![],
                    updated_mns: vec![
                        (
                            changed_hash,
                            DMNStateDiff {
                                service: Some(new_service),
                                ..empty_state_diff()
                            },
                        ),
                        (
                            payment_only_hash,
                            DMNStateDiff {
                                last_paid_height: Some(2_129_183),
                                consecutive_payments: Some(1),
                                pose_penalty: Some(0),
                                ..empty_state_diff()
                            },
                        ),
                    ],
                })
            });

        platform
            .update_state_masternode_list_v1(&mut state, 1, false)
            .expect("update must succeed");

        assert_eq!(state.full_masternode_list()[&payment_only_hash], unchanged);
        assert_eq!(state.hpmn_masternode_list()[&payment_only_hash], unchanged);
        assert_eq!(
            state.full_masternode_list()[&changed_hash].state.service,
            new_service
        );
        assert_eq!(
            state.hpmn_masternode_list()[&changed_hash].state.service,
            new_service
        );
        assert_eq!(
            state.masternode_changes.upserted,
            BTreeSet::from([changed_hash]),
            "payment-only updates must not cause writes alongside a real change"
        );
        assert!(state.masternode_changes.removed.is_empty());
        assert!(!state.masternode_changes.rewrite_all);
    }

    /// The IP every test evonode is registered with.
    const SERVICE_IP: &str = "1.2.3.4";
    const EVONODE_PRO_TX_HASH: [u8; 32] = [0x77u8; 32];
    const VALIDATOR_QUORUM_HASH: [u8; 32] = [0x55u8; 32];

    fn evonode_pro_tx_hash() -> ProTxHash {
        ProTxHash::from_byte_array(EVONODE_PRO_TX_HASH)
    }

    /// An evonode with platform ports 36656 (P2P) and 443 (HTTPS), as Core
    /// lists it with `-deprecatedrpc=service`: the legacy port fields carry
    /// the live ports and `addresses` repeats them.
    fn evonode() -> MasternodeListItem {
        let mut item = masternode(evonode_pro_tx_hash());
        item.node_type = MasternodeType::Evo;
        item.state.platform_node_id = Some([7u8; 20]);
        #[allow(deprecated)]
        {
            item.state.legacy_platform_p2p_port = Some(36656);
            item.state.legacy_platform_http_port = Some(443);
        }
        item.state.addresses = Some(MasternodeAddresses {
            core_p2p: vec!["1.2.3.4:1234".to_string()],
            platform_p2p: vec!["1.2.3.4:36656".to_string()],
            platform_https: vec!["1.2.3.4:443".to_string()],
        });
        item
    }

    /// The same evonode in a state whose legacy ports are absent: the ports
    /// are only in `addresses`.
    fn evonode_without_legacy_ports() -> MasternodeListItem {
        let mut item = evonode();
        #[allow(deprecated)]
        {
            item.state.legacy_platform_p2p_port = None;
            item.state.legacy_platform_http_port = None;
        }
        item
    }

    /// `item` as a node that restarted holds it: loaded back from storage.
    fn reloaded(item: MasternodeListItem) -> MasternodeListItem {
        MasternodeV0::from(item).into()
    }

    /// Runs one masternode list update per diff, in order, for a state
    /// holding `evonode` as a member of one validator set, with Core reporting
    /// the diffs for it, and returns the state afterwards.
    fn state_after_diffs(
        mut evonode: MasternodeListItem,
        diffs: Vec<DMNStateDiff>,
    ) -> PlatformState {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();

        let pro_tx_hash = evonode.pro_tx_hash;
        let quorum_hash = QuorumHash::from_byte_array(VALIDATOR_QUORUM_HASH);
        let mut state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        evonode.state = in_stored_form(evonode.state);
        state.insert_masternode(evonode);
        let validator = ValidatorV0 {
            pro_tx_hash,
            public_key: None,
            node_ip: SERVICE_IP.to_string(),
            node_id: PubkeyHash::from_byte_array([7u8; 20]),
            core_port: 1234,
            platform_http_port: 443,
            platform_p2p_port: 36656,
            is_banned: false,
        };
        let mut rng = StdRng::seed_from_u64(1);
        state.insert_validator_set(
            quorum_hash,
            ValidatorSet::V0(ValidatorSetV0 {
                quorum_hash,
                quorum_index: None,
                core_height: 1,
                members: BTreeMap::from([(pro_tx_hash, validator)]),
                threshold_public_key: SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key(),
            }),
        );
        state.mark_saved();

        let block_count = diffs.len() as u32;
        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |base_block, block| {
                Ok(MasternodeListDiff {
                    base_height: base_block.unwrap_or_default(),
                    block_height: block,
                    added_mns: vec![],
                    removed_mns: vec![],
                    updated_mns: vec![(pro_tx_hash, diffs[block as usize - 1].clone())],
                })
            });

        for block in 1..=block_count {
            platform
                .update_state_masternode_list_v1(&mut state, block, false)
                .expect("update must succeed");
        }

        state
    }

    /// The evonode's member of the validator set.
    fn validator_in(state: &PlatformState) -> ValidatorV0 {
        state.validator_sets()[&QuorumHash::from_byte_array(VALIDATOR_QUORUM_HASH)].members()
            [&evonode_pro_tx_hash()]
            .clone()
    }

    /// The validator a quorum Drive learns about later gets for the evonode,
    /// once from the state as held in memory and once as a node that
    /// restarted loads it back. Validator-set membership decides quorum
    /// rotation, so every node must build the same one.
    fn validators_for_a_new_quorum(state: &PlatformState) -> [Option<ValidatorV0>; 2] {
        let pro_tx_hash = evonode_pro_tx_hash();
        let mut restarted = state.clone();
        restarted.insert_masternode(reloaded(state.full_masternode_list()[&pro_tx_hash].clone()));
        [state, &restarted].map(|state| {
            ValidatorV0::new_validator_if_masternode_in_state(pro_tx_hash, None, state)
        })
    }

    /// Asserts that the evonode's member of the existing validator set has
    /// the given ports, and that a new quorum gets the same member.
    fn assert_validator_ports(state: &PlatformState, p2p_port: u16, http_port: u16) {
        let validator = validator_in(state);
        assert_eq!(validator.node_ip, SERVICE_IP);
        assert_eq!(validator.platform_p2p_port, p2p_port);
        assert_eq!(validator.platform_http_port, http_port);
        for new_quorum_validator in validators_for_a_new_quorum(state) {
            assert_eq!(new_quorum_validator.as_ref(), Some(&validator));
        }
    }

    /// The network info of the evonode after moving to platform ports 36657
    /// (P2P) and 8443 (HTTPS), as Core prints an ExtAddr one.
    fn moved_platform_ports() -> MasternodeAddresses {
        MasternodeAddresses {
            core_p2p: vec!["1.2.3.4:1234".to_string()],
            platform_p2p: vec!["1.2.3.4:36657".to_string()],
            platform_https: vec!["1.2.3.4:8443".to_string()],
        }
    }

    /// A node that restarts reloads its masternode lists from storage, and
    /// validator-set membership, which decides quorum rotation, must not
    /// depend on whether it did. So after every masternode list update the
    /// lists held in memory must be exactly what storage gives back. The
    /// updates: an ExtAddr evonode added with its platform ports in
    /// `addresses` and a payout list, a port move that only `addresses`
    /// carries, then a revoke that zeroes the legacy ports and carries no
    /// `addresses`.
    #[test]
    fn should_hold_after_every_update_the_masternode_lists_a_restarted_node_loads() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig {
                testing_configs: PlatformTestConfig {
                    store_platform_state: true,
                    ..PlatformTestConfig::default_minimal_verifications()
                },
                ..Default::default()
            })
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut added = evonode();
        added.collateral_address = Some([0x21; 20]);
        added.state.owner_address = Some([0x22; 20]);
        added.state.payout_address = None;
        added.state.payouts = Some(vec![DMNPayout {
            address: [0x23; 20],
            script: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0x23; 20])),
            reward: 10000,
        }]);
        let pro_tx_hash = added.pro_tx_hash;
        let port_move = DMNStateDiff {
            addresses: Some(Some(moved_platform_ports())),
            ..empty_state_diff()
        };
        #[allow(deprecated)]
        let revoke = DMNStateDiff {
            service: Some("[::]:0".parse().expect("socket address")),
            pose_ban_height: Some(Some(3)),
            revocation_reason: Some(1),
            platform_node_id: Some([0u8; 20]),
            pub_key_operator: Some(vec![1u8; 48]),
            legacy_platform_p2p_port: Some(0),
            legacy_platform_http_port: Some(0),
            ..empty_state_diff()
        };
        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |base_block, block| {
                let (added_mns, updated_mns) = match block {
                    1 => (vec![added.clone()], vec![]),
                    2 => (vec![], vec![(pro_tx_hash, port_move.clone())]),
                    _ => (vec![], vec![(pro_tx_hash, revoke.clone())]),
                };
                Ok(MasternodeListDiff {
                    base_height: base_block.unwrap_or_default(),
                    block_height: block,
                    added_mns,
                    removed_mns: vec![],
                    updated_mns,
                })
            });

        let transaction = platform.drive.grove.start_transaction();
        let mut state = platform.state.load().as_ref().clone();
        // The ports and ban each update leaves the evonode with.
        let expected = [
            (Some(36656), Some(443), false),
            (Some(36657), Some(8443), false),
            (Some(0), Some(0), true),
        ];
        for (block, (p2p, http, is_banned)) in (1..).zip(expected) {
            platform
                .update_state_masternode_list_v1(&mut state, block, false)
                .expect("update must succeed");
            platform
                .store_platform_state(&state, Some(&transaction), platform_version)
                .expect("store platform state");
            state.mark_saved();
            let reloaded = Platform::<MockCoreRPCLike>::fetch_platform_state(
                &platform.drive,
                Some(&transaction),
                platform_version,
            )
            .expect("fetch platform state")
            .expect("a state was stored");

            assert_eq!(
                reloaded.full_masternode_list(),
                state.full_masternode_list(),
                "the full list after block {block}"
            );
            assert_eq!(
                reloaded.hpmn_masternode_list(),
                state.hpmn_masternode_list(),
                "the evonode list after block {block}"
            );
            let evonode = &reloaded.hpmn_masternode_list()[&pro_tx_hash].state;
            assert_eq!(
                (
                    PlatformPorts::of(evonode),
                    evonode.pose_ban_height.is_some()
                ),
                (PlatformPorts { p2p, http }, is_banned),
                "the evonode after block {block}"
            );
        }
    }

    /// An ExtAddr evonode keeps its platform ports in its network info and its
    /// legacy port fields at zero, so a ProUpServTx that only moves its ports
    /// reaches Drive as a diff whose only network field is `addresses`
    /// (`service` too only with `-deprecatedrpc=service`). Tenderdash must be
    /// told the new port, or it keeps dialling the old one.
    #[test]
    fn should_update_validator_ports_from_an_addresses_only_diff() {
        let state = state_after_diffs(
            evonode_without_legacy_ports(),
            vec![DMNStateDiff {
                addresses: Some(Some(moved_platform_ports())),
                ..empty_state_diff()
            }],
        );

        assert_validator_ports(&state, 36657, 8443);
    }

    /// The same port move as Core prints it with `-deprecatedrpc=service`:
    /// `service` repeats because the network info changed, while the legacy
    /// port fields are absent because they did not change.
    #[test]
    fn should_update_validator_ports_from_addresses_when_the_diff_also_carries_service() {
        let state = state_after_diffs(
            evonode(),
            vec![DMNStateDiff {
                service: Some("1.2.3.4:1234".parse().expect("socket address")),
                addresses: Some(Some(moved_platform_ports())),
                ..empty_state_diff()
            }],
        );

        assert_validator_ports(&state, 36657, 8443);
    }

    /// An ExtAddr evonode that moves only its HTTPS port lists its whole
    /// network info again, the unchanged P2P entry included.
    #[test]
    fn should_update_the_validator_https_port_when_only_it_moved() {
        let state = state_after_diffs(
            evonode(),
            vec![DMNStateDiff {
                service: Some("1.2.3.4:1234".parse().expect("socket address")),
                addresses: Some(Some(MasternodeAddresses {
                    core_p2p: vec!["1.2.3.4:1234".to_string()],
                    platform_p2p: vec!["1.2.3.4:36656".to_string()],
                    platform_https: vec!["1.2.3.4:8443".to_string()],
                })),
                ..empty_state_diff()
            }],
        );

        assert_validator_ports(&state, 36656, 8443);
    }

    /// HTTPS alone is not an existing-validator refresh trigger. New quorums
    /// still use the updated stored HTTPS port.
    #[test]
    fn should_keep_the_existing_validator_https_port_without_a_refresh_trigger() {
        let state = state_after_diffs(
            evonode(),
            vec![DMNStateDiff {
                addresses: Some(Some(MasternodeAddresses {
                    core_p2p: vec![],
                    platform_p2p: vec![],
                    platform_https: vec!["1.2.3.4:8443".to_string()],
                })),
                ..empty_state_diff()
            }],
        );
        assert_eq!(validator_in(&state).platform_http_port, 443);
        for validator in validators_for_a_new_quorum(&state) {
            assert_eq!(validator.expect("new validator").platform_http_port, 8443);
        }
    }

    /// When a legacy evonode changes only a platform port, the diff does not
    /// carry the node's IP, so Core pairs the new port in `addresses` with the
    /// placeholder host 255.255.255.255. Only the port may be taken from it.
    #[test]
    fn should_not_take_the_placeholder_host_from_a_legacy_evonode_port_diff() {
        let state = state_after_diffs(
            evonode(),
            vec![DMNStateDiff {
                #[allow(deprecated)]
                legacy_platform_p2p_port: Some(36657),
                addresses: Some(Some(MasternodeAddresses {
                    core_p2p: vec![],
                    platform_p2p: vec!["255.255.255.255:36657".to_string()],
                    platform_https: vec![],
                })),
                ..empty_state_diff()
            }],
        );

        assert_validator_ports(&state, 36657, 443);
    }

    /// A legacy evonode's diff lists in `addresses` only the entries that
    /// changed, next to the changed legacy port field. A later diff that
    /// changes the other port must not undo an earlier one's port.
    #[test]
    fn should_keep_each_platform_port_from_diffs_that_list_one_entry_each() {
        let state = state_after_diffs(
            evonode(),
            vec![
                DMNStateDiff {
                    #[allow(deprecated)]
                    legacy_platform_p2p_port: Some(36657),
                    addresses: Some(Some(MasternodeAddresses {
                        core_p2p: vec![],
                        platform_p2p: vec!["255.255.255.255:36657".to_string()],
                        platform_https: vec![],
                    })),
                    ..empty_state_diff()
                },
                DMNStateDiff {
                    #[allow(deprecated)]
                    legacy_platform_http_port: Some(8443),
                    addresses: Some(Some(MasternodeAddresses {
                        core_p2p: vec![],
                        platform_p2p: vec![],
                        platform_https: vec!["255.255.255.255:8443".to_string()],
                    })),
                    ..empty_state_diff()
                },
            ],
        );

        assert_eq!(validator_in(&state).platform_p2p_port, 36657);
        for new_quorum_validator in validators_for_a_new_quorum(&state) {
            let new_quorum_validator = new_quorum_validator.expect("the evonode has ports");
            assert_eq!(new_quorum_validator.platform_p2p_port, 36657);
            assert_eq!(new_quorum_validator.platform_http_port, 8443);
        }
    }

    /// A diff of a legacy evonode's core address carries `addresses` with the
    /// core entry only, so the platform ports must not be read as cleared.
    #[test]
    fn should_keep_validator_ports_when_a_diff_moves_only_the_core_address() {
        let state = state_after_diffs(
            evonode(),
            vec![DMNStateDiff {
                service: Some("5.6.7.8:1234".parse().expect("socket address")),
                addresses: Some(Some(MasternodeAddresses {
                    core_p2p: vec!["5.6.7.8:1234".to_string()],
                    platform_p2p: vec![],
                    platform_https: vec![],
                })),
                ..empty_state_diff()
            }],
        );

        let validator = validator_in(&state);
        assert_eq!(validator.node_ip, "5.6.7.8");
        assert_eq!(validator.platform_p2p_port, 36656);
        assert_eq!(validator.platform_http_port, 443);
        for new_quorum_validator in validators_for_a_new_quorum(&state) {
            assert_eq!(new_quorum_validator.as_ref(), Some(&validator));
        }
    }

    /// A node that restarted holds the evonode without `addresses`. A diff
    /// that lists only the core entry must leave its platform ports as they
    /// are, as on a node that kept its state in memory.
    #[test]
    fn should_keep_validator_ports_after_a_restart_when_a_diff_moves_only_the_core_address() {
        let state = state_after_diffs(
            reloaded(evonode()),
            vec![DMNStateDiff {
                service: Some("1.2.3.4:1234".parse().expect("socket address")),
                addresses: Some(Some(MasternodeAddresses {
                    core_p2p: vec!["1.2.3.4:1234".to_string()],
                    platform_p2p: vec![],
                    platform_https: vec![],
                })),
                ..empty_state_diff()
            }],
        );

        assert_validator_ports(&state, 36656, 443);
    }

    /// After V24 a legacy evonode's revoke, or an operator key change, makes
    /// it ExtAddr with an empty network info: Core prints its platform ports
    /// as 0 and no `addresses`. Earlier binaries accept that diff, so every
    /// node must build the validator they build from the legacy fields: a
    /// banned member on the unspecified IP with ports 0. A node that kept the
    /// evonode's `addresses` in memory and a node that restarted must both
    /// build it, because validator-set membership decides quorum rotation.
    #[test]
    fn should_agree_on_the_validator_after_a_legacy_evonode_is_revoked_whether_or_not_the_node_restarted(
    ) {
        #[allow(deprecated)]
        let revoke = DMNStateDiff {
            service: Some("[::]:0".parse().expect("socket address")),
            pose_ban_height: Some(Some(2)),
            revocation_reason: Some(1),
            platform_node_id: Some([0u8; 20]),
            pub_key_operator: Some(vec![1u8; 48]),
            legacy_platform_p2p_port: Some(0),
            legacy_platform_http_port: Some(0),
            ..empty_state_diff()
        };
        let kept = state_after_diffs(evonode(), vec![revoke.clone()]);
        let restarted = state_after_diffs(reloaded(evonode()), vec![revoke]);

        let pro_tx_hash = evonode_pro_tx_hash();
        let new_quorum_validator = ValidatorV0 {
            pro_tx_hash,
            public_key: None,
            node_ip: "::".to_string(),
            node_id: PubkeyHash::from_byte_array([0u8; 20]),
            core_port: 0,
            platform_http_port: 0,
            platform_p2p_port: 0,
            is_banned: true,
        };
        // A refresh of an existing member takes the ban, the IP and the ports
        // from the diff, and keeps its node id and core port.
        let refreshed_member = ValidatorV0 {
            node_id: PubkeyHash::from_byte_array([7u8; 20]),
            core_port: 1234,
            ..new_quorum_validator.clone()
        };
        for state in [&kept, &restarted] {
            assert_eq!(
                ValidatorV0::new_validator_if_masternode_in_state(pro_tx_hash, None, state),
                Some(new_quorum_validator.clone())
            );
            assert_eq!(validator_in(state), refreshed_member);
        }
        let stored = |state: &PlatformState| {
            MasternodeV0::from(state.full_masternode_list()[&pro_tx_hash].clone()).state
        };
        assert_eq!(stored(&kept), stored(&restarted));
    }

    /// When Core resets an ExtAddr evonode's operator fields (ProUpRevTx, or
    /// a ProUpRegTx that changes the operator key), its network info becomes
    /// empty: the diff has `service` `[::]:0` and no `addresses` key, so the
    /// platform ports keep resolving to the old endpoint. That endpoint is
    /// never advertised, because Core bans the masternode in the same
    /// transition and only a ProUpServTx, which sends new `addresses`, revives
    /// it. The evonode stays a member, banned, with the ports it last had.
    #[test]
    fn should_not_advertise_an_evonode_whose_network_info_core_emptied() {
        let state = state_after_diffs(
            evonode_without_legacy_ports(),
            vec![DMNStateDiff {
                service: Some("[::]:0".parse().expect("socket address")),
                pose_ban_height: Some(Some(2)),
                revocation_reason: Some(1),
                platform_node_id: Some([0u8; 20]),
                pub_key_operator: Some(vec![1u8; 48]),
                ..empty_state_diff()
            }],
        );

        let validator_set =
            &state.validator_sets()[&QuorumHash::from_byte_array(VALIDATOR_QUORUM_HASH)];
        let new_quorum_validator = ValidatorV0 {
            pro_tx_hash: evonode_pro_tx_hash(),
            public_key: None,
            node_ip: "::".to_string(),
            node_id: PubkeyHash::from_byte_array([0u8; 20]),
            core_port: 0,
            platform_http_port: 443,
            platform_p2p_port: 36656,
            is_banned: true,
        };
        assert_eq!(
            validator_set.members()[&evonode_pro_tx_hash()],
            ValidatorV0 {
                node_id: PubkeyHash::from_byte_array([7u8; 20]),
                core_port: 1234,
                ..new_quorum_validator.clone()
            }
        );
        assert!(
            validator_set.to_update().validator_updates.is_empty(),
            "a banned validator is left out of the validator set update"
        );
        for validator in validators_for_a_new_quorum(&state) {
            assert_eq!(
                validator,
                Some(new_quorum_validator.clone()),
                "a validator set built later gets the same banned member"
            );
        }
    }
}

#[cfg(test)]
mod port_resolution_tests {
    use super::in_stored_form;
    use crate::config::PlatformConfig;
    use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
    use crate::platform_types::validator::v0::{NewValidatorIfMasternodeInState, ValidatorV0};
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{Network, Txid};
    use dpp::dashcore::{ProTxHash, PubkeyHash};
    use dpp::dashcore_rpc::json::{DMNState, MasternodeListItem};
    use dpp::dashcore_rpc::json::{MasternodeAddresses, MasternodeType};
    use dpp::version::PlatformVersion;

    const PRO_TX_HASH: [u8; 32] = [0x77u8; 32];

    /// An evonode registered at 1.2.3.4 with the given legacy platform ports
    /// and nested addresses.
    fn evonode_state(
        legacy_platform_p2p_port: Option<u32>,
        legacy_platform_http_port: Option<u32>,
        addresses: Option<MasternodeAddresses>,
    ) -> DMNState {
        #[allow(deprecated)]
        DMNState {
            service: "1.2.3.4:9999".parse().expect("socket address"),
            registered_height: 0,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: 0,
            owner_address: Some([0u8; 20]),
            voting_address: [0u8; 20],
            payout_address: Some([0u8; 20]),
            payouts: None,
            pub_key_operator: vec![0u8; 48],
            operator_payout_address: None,
            platform_node_id: Some([7u8; 20]),
            legacy_platform_p2p_port,
            legacy_platform_http_port,
            addresses,
        }
    }

    fn platform_addresses(p2p: &str, https: &str) -> MasternodeAddresses {
        MasternodeAddresses {
            core_p2p: vec!["1.2.3.4:9999".to_string()],
            platform_p2p: vec![p2p.to_string()],
            platform_https: vec![https.to_string()],
        }
    }

    fn validator_for(dmn_state: DMNState) -> Option<ValidatorV0> {
        let platform_version = PlatformVersion::latest();
        let mut state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        let pro_tx_hash = ProTxHash::from_byte_array(PRO_TX_HASH);
        state.insert_masternode(MasternodeListItem {
            node_type: MasternodeType::Evo,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array([0u8; 32]),
            collateral_index: 0,
            collateral_address: Some([0u8; 20]),
            operator_reward: 0.0,
            state: in_stored_form(dmn_state),
        });
        ValidatorV0::new_validator_if_masternode_in_state(pro_tx_hash, None, &state)
    }

    /// An ExtAddr evonode keeps its platform ports in its network info; its
    /// legacy port fields stay zero.
    #[test]
    fn should_take_platform_ports_from_addresses_when_legacy_ports_are_zero() {
        let validator = validator_for(evonode_state(
            Some(0),
            Some(0),
            Some(platform_addresses("1.2.3.4:36656", "1.2.3.4:443")),
        ))
        .expect("the evonode has platform ports");

        assert_eq!(validator.platform_p2p_port, 36656);
        assert_eq!(validator.platform_http_port, 443);
    }

    /// A state whose legacy ports are absent takes its ports from
    /// `addresses`.
    #[test]
    fn should_take_platform_ports_from_addresses_when_legacy_ports_are_absent() {
        let validator = validator_for(evonode_state(
            None,
            None,
            Some(platform_addresses("1.2.3.4:36656", "1.2.3.4:443")),
        ))
        .expect("the evonode has platform ports");

        assert_eq!(validator.platform_p2p_port, 36656);
        assert_eq!(validator.platform_http_port, 443);
    }

    /// The validator is advertised on the core IP from `service`, whatever
    /// host `addresses` names for the platform services: a legacy evonode's
    /// port-only diff pairs the port with the placeholder host
    /// 255.255.255.255.
    #[test]
    fn should_advertise_the_service_ip_whatever_host_addresses_name() {
        let validator = validator_for(evonode_state(
            Some(36656),
            Some(443),
            Some(platform_addresses(
                "255.255.255.255:36656",
                "203.0.113.7:443",
            )),
        ))
        .expect("the evonode has platform ports");

        assert_eq!(validator.node_ip, "1.2.3.4");
        assert_eq!(validator.core_port, 9999);
    }

    /// Core before v23 lists no nested addresses; the legacy ports apply.
    #[test]
    fn should_take_legacy_ports_without_addresses() {
        let validator = validator_for(evonode_state(Some(26656), Some(443), None))
            .expect("the evonode has platform ports");

        assert_eq!(validator.node_ip, "1.2.3.4");
        assert_eq!(validator.platform_p2p_port, 26656);
        assert_eq!(validator.platform_http_port, 443);
    }

    /// Validator-set membership decides quorum rotation, so it must not
    /// depend on how a node reads a port: an evonode whose legacy ports are 0
    /// and that lists no nested ports stays a validator, with those ports.
    #[test]
    fn should_keep_an_evonode_with_zero_legacy_ports_as_a_validator() {
        let validator = validator_for(evonode_state(Some(0), Some(0), None))
            .expect("membership does not depend on the port value");

        assert_eq!(validator.platform_p2p_port, 0);
        assert_eq!(validator.platform_http_port, 0);
    }

    /// A revoked legacy evonode as Core lists it with
    /// `-deprecatedrpc=service`: its network info is empty, so `service` is
    /// `[::]:0` and `addresses` has no platform entry, while the legacy port
    /// fields keep the ports it was registered with.
    fn revoked_legacy_evonode_state(addresses: &str) -> DMNState {
        serde_json::from_str(&format!(
            r#"{{
                "service": "[::]:0",
                "addresses": {addresses},
                "registeredHeight": 100,
                "PoSeRevivedHeight": -1,
                "PoSeBanHeight": 200,
                "revocationReason": 1,
                "ownerAddress": "yPBWCdMRY5PsS3hJzs7csbdWQVRR85yxUz",
                "votingAddress": "ySM11LUD65Bi4p1gm68XLkdWc65TBKRzvQ",
                "payoutAddress": "yX4Ve7Q8Y4jscV4LZJD8HVCHKyePzR3MhA",
                "pubKeyOperator": "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "platformNodeID": "0000000000000000000000000000000000000000",
                "platformP2PPort": 26656,
                "platformHTTPPort": 443
            }}"#
        ))
        .expect("Core's JSON for a revoked evonode")
    }

    /// Whether an evonode is a validator decides quorum rotation, so every
    /// node must build the same member for it. For an evonode whose listed
    /// `addresses` hold no platform entry, that is the member the legacy
    /// port fields alone give: banned, on the unspecified IP, with its
    /// registered ports.
    #[test]
    fn should_build_a_revoked_legacy_evonode_from_its_legacy_ports() {
        for addresses in [r#"{}"#, r#"{"core_p2p": []}"#] {
            let validator = validator_for(revoked_legacy_evonode_state(addresses));

            assert_eq!(
                validator,
                Some(ValidatorV0 {
                    pro_tx_hash: ProTxHash::from_byte_array(PRO_TX_HASH),
                    public_key: None,
                    node_ip: "::".to_string(),
                    node_id: PubkeyHash::from_byte_array([0u8; 20]),
                    core_port: 0,
                    platform_http_port: 443,
                    platform_p2p_port: 26656,
                    is_banned: true,
                }),
                "addresses: {addresses}"
            );
        }
    }
}
