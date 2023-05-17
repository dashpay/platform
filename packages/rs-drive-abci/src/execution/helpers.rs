use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::quorum::Quorum;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformState;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::ProTxHash;
use dashcore_rpc::dashcore_rpc_json::MasternodeListDiff;
use dashcore_rpc::json::{MasternodeListItem, MasternodeType};
use dpp::block::block_info::BlockInfo;
use drive::grovedb::Transaction;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// Represents the outcome of an attempt to update the state of a masternode list.
pub struct UpdateStateMasternodeListOutcome {
    /// The diff between two masternode lists.
    masternode_list_diff: MasternodeListDiff,
    /// The set of ProTxHashes that correspond to masternodes that were deleted from the list.
    removed_masternodes: BTreeMap<ProTxHash, MasternodeListItem>,
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

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Retrieves the genesis time for the specified block height and block time.
    ///
    /// # Arguments
    ///
    /// * `block_height` - The block height for which to retrieve the genesis time.
    /// * `block_time_ms` - The block time in milliseconds.
    /// * `transaction` - A reference to the transaction.
    ///
    /// # Returns
    ///
    /// * `Result<u64, Error>` - The genesis time as a `u64` value on success, or an `Error` on failure.
    pub(crate) fn get_genesis_time(
        &self,
        block_height: u64,
        block_time_ms: u64,
        transaction: &Transaction,
    ) -> Result<u64, Error> {
        if block_height == self.config.abci.genesis_height {
            // we do not set the genesis time to the cache here,
            // instead that must be done after finalizing the block
            Ok(block_time_ms)
        } else {
            //todo: lazy load genesis time
            self.drive
                .get_genesis_time(Some(transaction))
                .map_err(Error::Drive)?
                .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                    "the genesis time must be set",
                )))
        }
    }

    /// Updates the quorum information for the platform state based on the given core block height.
    ///
    /// # Arguments
    ///
    /// * `state` - A mutable reference to the platform state.
    /// * `core_block_height` - The core block height for which to update the quorum information.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, ExecutionError>` - A `SimpleConsensusValidationResult`
    ///   on success, or an `Error` on failure.
    pub(crate) fn update_quorum_info(
        &self,
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        start_from_scratch: bool,
    ) -> Result<(), Error> {
        if !start_from_scratch && core_block_height == block_platform_state.core_height() {
            tracing::debug!(
                method = "update_quorum_info",
                "no update quorum at height {}",
                core_block_height
            );
            return Ok(()); // no need to do anything
        }
        tracing::debug!(
            method = "update_quorum_info",
            "update of quorums for height {}",
            core_block_height
        );
        let quorum_list = self
            .core_rpc
            .get_quorum_listextended(Some(core_block_height))?;
        let quorum_info = quorum_list
            .quorums_by_type
            .get(&self.config.quorum_type())
            .ok_or(Error::Execution(ExecutionError::DashCoreBadResponseError(
                format!(
                    "expected quorums of type {}, but did not receive any from Dash Core",
                    self.config.quorum_type
                ),
            )))?;

        tracing::debug!(
            method = "update_quorum_info",
            "old {:?}",
            block_platform_state.validator_sets
        );

        tracing::debug!(
            method = "update_quorum_info",
            "new quorum_info {:?}",
            quorum_info
        );

        // Remove validator_sets entries that are no longer valid for the core block height
        block_platform_state
            .validator_sets
            .retain(|key, _| quorum_info.contains_key(key));

        // Fetch quorum info results and their keys from the RPC
        let mut quorum_infos = quorum_info
            .iter()
            .filter(|(key, _)| {
                !block_platform_state
                    .validator_sets
                    .contains_key(key.as_ref())
            })
            .map(|(key, _)| {
                let quorum_info_result =
                    self.core_rpc
                        .get_quorum_info(self.config.quorum_type(), key, None)?;
                Ok((*key, quorum_info_result))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // Sort by height and then by hash
        quorum_infos.sort_by(|a, b| {
            let height_cmp = a.1.height.cmp(&b.1.height);
            if height_cmp == std::cmp::Ordering::Equal {
                a.0.cmp(&b.0) // Compare hashes if heights are equal
            } else {
                height_cmp
            }
        });

        // Map to quorums
        let new_quorums = quorum_infos
            .into_iter()
            .map(|(key, info_result)| {
                let quorum = Quorum::try_from_info_result(info_result, block_platform_state)?;
                Ok((key, quorum))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        // Add new validator_sets entries
        block_platform_state
            .validator_sets
            .extend(new_quorums.into_iter());

        block_platform_state
            .validator_sets
            .sort_by(|_, quorum_a, _, quorum_b| {
                let primary_comparison = quorum_b.core_height.cmp(&quorum_a.core_height);
                if primary_comparison == Ordering::Equal {
                    quorum_b
                        .quorum_hash
                        .cmp(&quorum_a.quorum_hash)
                        .then_with(|| quorum_b.core_height.cmp(&quorum_a.core_height))
                } else {
                    primary_comparison
                }
            });

        tracing::debug!(
            method = "update_quorum_info",
            "new {:?}",
            block_platform_state.validator_sets
        );

        block_platform_state.quorums_extended_info = quorum_list.quorums_by_type;
        Ok(())
    }

    pub(crate) fn update_state_masternode_list(
        &self,
        state: &mut PlatformState,
        core_block_height: u32,
        start_from_scratch: bool,
    ) -> Result<UpdateStateMasternodeListOutcome, Error> {
        let previous_core_height = if start_from_scratch {
            // baseBlock must be a chain height and not 0
            None
        } else {
            let state_core_height = state.core_height();
            if core_block_height == state_core_height {
                return Ok(UpdateStateMasternodeListOutcome::default()); // no need to do anything
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

        let updated_masternodes = updated_mns
            .iter()
            .map(|(pro_tx_hash, masternode)| (pro_tx_hash, masternode.clone()));

        updated_masternodes.for_each(|(pro_tx_hash, state_diff)| {
            if let Some(masternode_list_item) = state.full_masternode_list.get_mut(pro_tx_hash) {
                if let Some(masternode_list_item) = state.hpmn_masternode_list.get_mut(pro_tx_hash)
                {
                    masternode_list_item.state.apply_diff(state_diff.clone());
                }
                masternode_list_item.state.apply_diff(state_diff);
            }
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

        Ok(UpdateStateMasternodeListOutcome {
            masternode_list_diff: masternode_diff,
            removed_masternodes,
        })
    }

    /// Updates the masternode list in the platform state based on changes in the masternode list
    /// from Dash Core between two block heights.
    ///
    /// This function fetches the masternode list difference between the current core block height
    /// and the previous core block height, then updates the full masternode list and the
    /// HPMN (high performance masternode) list in the platform state accordingly.
    ///
    /// # Arguments
    ///
    /// * `state` - A mutable reference to the platform state to be updated.
    /// * `core_block_height` - The current block height in the Dash Core.
    /// * `transaction` - The current groveDB transaction.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns `Ok(())` if the update is successful. Returns an error if
    ///   there is a problem fetching the masternode list difference or updating the state.
    fn update_masternode_list(
        &self,
        platform_state: Option<&PlatformState>,
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        is_init_chain: bool,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        if let Some(last_commited_block_info) =
            block_platform_state.last_committed_block_info.as_ref()
        {
            if core_block_height == last_commited_block_info.basic_info.core_height {
                tracing::debug!(
                    method = "update_masternode_list",
                    "no update mnl at height {}",
                    core_block_height
                );
                return Ok(()); // no need to do anything
            }
        }
        tracing::debug!(
            method = "update_masternode_list",
            "update mnl to height {} at block {}",
            core_block_height,
            block_platform_state.core_height()
        );
        if block_platform_state.last_committed_block_info.is_some() || is_init_chain {
            let UpdateStateMasternodeListOutcome {
                masternode_list_diff,
                removed_masternodes,
            } = self.update_state_masternode_list(
                block_platform_state,
                core_block_height,
                is_init_chain,
            )?;

            self.update_masternode_identities(
                masternode_list_diff,
                &removed_masternodes,
                block_info,
                platform_state,
                transaction,
            )?;

            if !removed_masternodes.is_empty() {
                self.drive.remove_validators_proposed_app_versions(
                    removed_masternodes
                        .into_keys()
                        .map(|pro_tx_hash| pro_tx_hash.into_inner()),
                    Some(transaction),
                )?;
            }
        }

        Ok(())
    }

    /// Updates the core information in the platform state based on the given core block height.
    ///
    /// This function updates both the masternode list and the quorum information in the platform
    /// state. It calls the update_masternode_list and update_quorum_info functions to perform
    /// the respective updates.
    ///
    /// # Arguments
    ///
    /// * platform_state - A reference to the platform state before execution of current block.
    /// * block_platform_state - A mutable reference to the current platform state in the block
    /// execution context to be updated.
    /// * core_block_height - The current block height in the Dash Core.
    /// * is_init_chain - A boolean indicating if the chain is being initialized.
    /// * block_info - A reference to the block information.
    /// * transaction - The current groveDB transaction.
    ///
    /// # Returns
    ///
    /// * Result<(), Error> - Returns Ok(()) if the update is successful. Returns an error if
    /// there is a problem updating the masternode list, quorum information, or the state.
    pub(crate) fn update_core_info(
        &self,
        platform_state: Option<&PlatformState>,
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        is_init_chain: bool,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        self.update_masternode_list(
            platform_state,
            block_platform_state,
            core_block_height,
            is_init_chain,
            block_info,
            transaction,
        )?;

        self.update_quorum_info(block_platform_state, core_block_height, false)
    }
}
