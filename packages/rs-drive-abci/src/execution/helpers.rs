use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformState;
use dashcore::signer::sign;
use dashcore_rpc::dashcore_rpc_json::{ProTxHash, QuorumHash};
use dashcore_rpc::json::{QuorumInfoResult, QuorumType};
use dpp::bls_signatures;
use dpp::bls_signatures::Serialize;
use dpp::validation::{SimpleConsensusValidationResult, SimpleValidationResult, ValidationResult};
use drive::grovedb::Transaction;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use tenderdash_abci::proto::abci::CommitInfo;
use tenderdash_abci::proto::types::BlockId;

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
            .filter(|(key, _)| !state.validator_sets.contains_key(key))
            .map(|(key, _)| {
                let quorum_info_result =
                    self.core_rpc
                        .get_quorum_info(self.config.quorum_type, key, None)?;
                Ok((key.clone(), quorum_info_result))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // Add new validator_sets entries
        state.validator_sets.extend(new_quorums.into_iter());

        state.quorums_extended_info = quorum_list.quorums_by_type;
        return Ok(());
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
    pub(crate) fn update_masternode_list(
        &self,
        state: &mut PlatformState,
        core_block_height: u32,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        let previous_core_height = state.core_height();
        if core_block_height == previous_core_height {
            return Ok(()); // no need to do anything
        }

        let masternode_list_diff = self
            .core_rpc
            .get_protx_diff(previous_core_height, core_block_height)?;
        //todo: clean up
        let updated_masternodes = masternode_list_diff.mn_list.into_iter().map(|masternode| {
            let pro_tx_hash =
                ProTxHash::from(hex::encode(masternode.pro_reg_tx_hash.clone()).as_str());
            (pro_tx_hash, masternode)
        });

        //filter updated masternodes between hpmns and non hpmns

        state
            .full_masternode_list
            .extend(updated_masternodes.clone());
        //FIXME: Filter updated masternodes for HPMNs
        state.hpmn_masternode_list.extend(updated_masternodes);

        let deleted_masternodes = masternode_list_diff
            .deleted_mns
            .into_iter()
            .map(|masternode| {
                let pro_tx_hash =
                    ProTxHash::from(hex::encode(masternode.pro_reg_tx_hash.clone()).as_str());
                pro_tx_hash
            })
            .collect::<BTreeSet<ProTxHash>>();

        state
            .hpmn_masternode_list
            .retain(|key, _| !deleted_masternodes.contains(key));
        state
            .full_masternode_list
            .retain(|key, _| !deleted_masternodes.contains(key));

        //Todo: masternode identities

        //For all deleted masternodes we need to remove them from the state of the app version votes

        self.drive.remove_validators_proposed_app_versions(
            deleted_masternodes
                .into_iter()
                .map(|a| a.0.try_into().unwrap()),
            Some(transaction),
        )?;

        Ok(())
    }
}
