use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformState;
use dashcore::signer::sign;
use dashcore_rpc::dashcore_rpc_json::QuorumHash;
use dashcore_rpc::json::{QuorumInfoResult, QuorumType};
use dpp::bls_signatures;
use dpp::bls_signatures::Serialize;
use dpp::validation::{SimpleConsensusValidationResult, SimpleValidationResult, ValidationResult};
use drive::grovedb::Transaction;
use std::collections::HashMap;
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
}
