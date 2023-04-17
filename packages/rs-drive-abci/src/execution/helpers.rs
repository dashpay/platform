use dashcore::hashes::Hash;
use dashcore::ProTxHash;
use std::collections::BTreeSet;

use dashcore_rpc::json::{MasternodeListDiffWithMasternodes, MasternodeType};
use drive::drive::block_info::BlockInfo;
use drive::grovedb::Transaction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::quorum::Quorum;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformState;

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
        if block_height == self.config.abci.genesis_height as u64 {
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
        state: &mut PlatformState,
        core_block_height: u32,
    ) -> Result<(), Error> {
        if core_block_height == state.core_height() {
            return Ok(()); // no need to do anything
        }

        let quorum_list = self
            .core_rpc
            .get_quorum_listextended(Some(core_block_height))?;
        let quorum_info = quorum_list
            .quorums_by_type
            .get(&self.config.quorum_type)
            .ok_or(Error::Execution(ExecutionError::DashCoreBadResponseError(
                format!(
                    "expected quorums of type {}, but did not receive any from Dash Core",
                    self.config.quorum_type
                ),
            )))?;

        // Remove validator_sets entries that are no longer valid for the core block height
        state
            .validator_sets
            .retain(|key, _| quorum_info.contains_key(key));

        let mut new_quorums = quorum_info
            .iter()
            .filter(|(key, _)| !state.validator_sets.contains_key(key.as_ref()))
            .map(|(key, _)| {
                let quorum_info_result =
                    self.core_rpc
                        .get_quorum_info(self.config.quorum_type, key, None)?;
                let quorum: Quorum = quorum_info_result.try_into()?;
                Ok((key.clone(), quorum))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // Add new validator_sets entries
        state.validator_sets.extend(new_quorums.into_iter());

        state.quorums_extended_info = quorum_list.quorums_by_type;
        return Ok(());
    }

    // TODO: re-enable

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
    pub(crate) fn update_masternode_list(
        &self,
        state: &mut PlatformState,
        core_block_height: u32,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        let previous_core_height = state.core_height();
        if core_block_height == previous_core_height {
            return Ok(()); // no need to do anything
        }

        let masternode_diff = self
            .core_rpc
            .get_protx_diff_with_masternodes(previous_core_height, core_block_height)?;

        let MasternodeListDiffWithMasternodes {
            added_mns,
            removed_mns,
            updated_mns,
            ..
        } = &masternode_diff;

        //todo: clean up
        let added_hpmns = added_mns.iter().filter_map(|masternode| {
            if masternode.node_type == MasternodeType::HighPerformance {
                Some((masternode.protx_hash.clone(), masternode.clone()))
            } else {
                None
            }
        });

        state.hpmn_masternode_list.extend(added_hpmns.clone());

        let added_masternodes = added_mns
            .iter()
            .map(|masternode| (masternode.protx_hash.clone(), masternode.clone()));

        state.full_masternode_list.extend(added_masternodes);

        let updated_masternodes = updated_mns
            .iter()
            .map(|masternode| (masternode.protx_hash.clone(), masternode.state_diff.clone()));

        updated_masternodes.for_each(|(pro_tx_hash, state_diff)| {
            if let Some(masternode_list_item) = state.full_masternode_list.get_mut(&pro_tx_hash) {
                if let Some(masternode_list_item) = state.hpmn_masternode_list.get_mut(&pro_tx_hash)
                {
                    masternode_list_item.state.apply_diff(state_diff.clone());
                }
                masternode_list_item.state.apply_diff(state_diff);
            }
        });

        let deleted_masternodes = removed_mns
            .iter()
            .map(|masternode| {
                let pro_tx_hash = masternode.protx_hash;
                pro_tx_hash
            })
            .collect::<BTreeSet<ProTxHash>>();

        state
            .hpmn_masternode_list
            .retain(|key, _| !deleted_masternodes.contains(key));
        state
            .full_masternode_list
            .retain(|key, _| !deleted_masternodes.contains(key));

        // //Todo: masternode identities
        // self.update_masternode_identities(
        //     previous_core_height,
        //     core_block_height,
        //     &block_info,
        //     state,
        //     &transaction,
        // )?;

        //For all deleted masternodes we need to remove them from the state of the app version votes

        if !deleted_masternodes.is_empty() {
            self.drive.remove_validators_proposed_app_versions(
                deleted_masternodes.into_iter().map(|a| a.into_inner()),
                Some(transaction),
            )?;
        }

        Ok(())
    }
}
