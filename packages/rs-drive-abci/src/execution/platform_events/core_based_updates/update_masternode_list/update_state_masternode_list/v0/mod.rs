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
    DMNStateDiff, MasternodeListDiff, MasternodeListItem, MasternodeType,
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

                    if let Some(p2p_port) = dmn_state_diff.platform_p2p_port {
                        validator.platform_p2p_port = p2p_port as u16;
                    }

                    if let Some(http_port) = dmn_state_diff.platform_http_port {
                        validator.platform_http_port = http_port as u16;
                    }
                }
            });
    }

    /// Whether applying `updated_mns` would change any item of `masternode_list`.
    ///
    /// Core reports a masternode as updated whenever any field of its state
    /// moved, including the payment and penalty fields — last paid height,
    /// consecutive payments, PoSe penalty — that change on nearly every Core
    /// block and that the stored item does not keep. Applying each diff to a
    /// copy tells the two apart.
    fn any_stored_masternode_changes(
        masternode_list: &BTreeMap<ProTxHash, MasternodeListItem>,
        updated_mns: &[(ProTxHash, DMNStateDiff)],
    ) -> bool {
        updated_mns.iter().any(|(pro_tx_hash, state_diff)| {
            masternode_list.get(pro_tx_hash).is_some_and(|item| {
                let mut updated = item.state.clone();
                updated.apply_diff(state_diff.clone());
                updated != item.state
            })
        })
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

        // Core advances a block without any stored masternode field changing far
        // more often than not: on mainnet almost every block's diff is only the
        // payment fields of the masternode paid in it. Returning before the
        // first mutable borrow keeps the platform state clean, which is what
        // lets the block skip rewriting the full saved state (over a megabyte
        // on mainnet) to disk.
        if !start_from_scratch
            && added_mns.is_empty()
            && removed_mns.is_empty()
            && !Self::any_stored_masternode_changes(state.full_masternode_list(), updated_mns)
        {
            return Ok(
                update_state_masternode_list_outcome::v0::UpdateStateMasternodeListOutcome {
                    masternode_list_diff: masternode_diff,
                    removed_masternodes: BTreeMap::new(),
                },
            );
        }

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

#[cfg(test)]
mod tests {
    use crate::config::PlatformConfig;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{Network, ProTxHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNState, DMNStateDiff, MasternodeListDiff, MasternodeListItem, MasternodeType,
    };
    use dpp::version::PlatformVersion;

    fn masternode(pro_tx_hash: ProTxHash) -> MasternodeListItem {
        MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array([0u8; 32]),
            collateral_index: 0,
            collateral_address: [0u8; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:1234".parse().expect("socket address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: [0u8; 20],
                voting_address: [0u8; 20],
                payout_address: [0u8; 20],
                pub_key_operator: vec![0u8; 48],
                operator_payout_address: None,
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
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
            pub_key_operator: None,
            operator_payout_address: None,
            platform_node_id: None,
            platform_p2p_port: None,
            platform_http_port: None,
        }
    }

    /// A clean state holding one masternode, and a platform whose Core reports
    /// `state_diff` for it on the next block.
    fn state_with_one_masternode_and_diff(
        state_diff: DMNStateDiff,
    ) -> (
        crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        PlatformState,
        ProTxHash,
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();

        let pro_tx_hash = ProTxHash::from_byte_array([0x77u8; 32]);
        let mut state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        state
            .full_masternode_list_mut()
            .insert(pro_tx_hash, masternode(pro_tx_hash));
        state.heavy_fields_dirty = false;

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
            .update_state_masternode_list_v0(&mut state, 1, false)
            .expect("update must succeed");

        assert!(
            !state.heavy_fields_dirty,
            "a diff that changes no stored field must not dirty the state"
        );
        assert_eq!(state.full_masternode_list(), &before);
        assert!(state.full_masternode_list().contains_key(&pro_tx_hash));
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
            .update_state_masternode_list_v0(&mut state, 1, false)
            .expect("update must succeed");

        assert!(state.heavy_fields_dirty);
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
}
