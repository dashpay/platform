use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::Txid;

use dpp::block::epoch::Epoch;

use dpp::validation::ValidationResult;
use drive::error::Error::GroveDB;

use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::collections::BTreeMap;

use crate::abci::AbciError;
use crate::error::execution::ExecutionError;

use crate::error::Error;
use crate::execution::types::block_execution_context::v0::{
    BlockExecutionContextV0Getters, BlockExecutionContextV0MutableGetters,
    BlockExecutionContextV0Setters,
};
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_fees::v0::BlockFeesV0;
use crate::execution::types::block_state_info::v0::{
    BlockStateInfoV0Getters, BlockStateInfoV0Methods, BlockStateInfoV0Setters,
};
use crate::execution::types::{block_execution_context, block_state_info};

use crate::platform_types::block_execution_outcome;
use crate::platform_types::block_proposal;
use crate::platform_types::epoch_info::v0::{EpochInfoV0Getters, EpochInfoV0Methods};
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Runs a block proposal, either from process proposal or prepare proposal.
    ///
    /// This function takes a `BlockProposal` and a `Transaction` as input and processes the block
    /// proposal. It first validates the block proposal and then processes raw state transitions,
    /// withdrawal transactions, and block fees. It also updates the validator set.
    ///
    /// # Arguments
    ///
    /// * `block_proposal` - The block proposal to be processed.
    /// * `transaction` - The transaction associated with the block proposal.
    ///
    /// # Returns
    ///
    /// * `Result<ValidationResult<BlockExecutionOutcome, Error>, Error>` - If the block proposal is
    ///   successfully processed, it returns a `ValidationResult` containing the `BlockExecutionOutcome`.
    ///   If the block proposal processing fails, it returns an `Error`. Consensus errors are returned
    ///   in the `ValidationResult`, while critical system errors are returned in the `Result`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with processing the block
    /// proposal, updating the core info, processing raw state transitions, or processing block fees.
    ///
    pub(super) fn run_block_proposal_v0(
        &self,
        block_proposal: block_proposal::v0::BlockProposal,
        epoch_info: EpochInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<ValidationResult<block_execution_outcome::v0::BlockExecutionOutcome, Error>, Error>
    {
        // Start by getting information from the state
        let state = self.state.read().unwrap();

        if tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(
                method = "run_block_proposal_v0",
                ?block_proposal,
                ?epoch_info,
                platform_state_fingerprint = hex::encode(state.fingerprint()),
                "running a block proposal on epoch {}",
                epoch_info.current_epoch_index()
            );
        }

        let last_block_time_ms = state.last_block_time_ms();
        let last_block_height =
            state.known_height_or(self.config.abci.genesis_height.saturating_sub(1));
        let last_block_core_height =
            state.known_core_height_or(self.config.abci.genesis_core_height);
        let hpmn_list_len = state.hpmn_list_len();

        let mut block_platform_state = state.clone();

        // Init block execution context
        let block_state_info = block_state_info::v0::BlockStateInfoV0::from_block_proposal(
            &block_proposal,
            last_block_time_ms,
        );

        // First let's check that this is the follower to a previous block
        if !block_state_info.next_block_to(last_block_height, last_block_core_height)? {
            // we are on the wrong height or round
            return Ok(ValidationResult::new_with_error(AbciError::WrongBlockReceived(format!(
                "received a block proposal for height: {} core height: {}, current height: {} core height: {}",
                block_state_info.height, block_state_info.core_chain_locked_height, last_block_height, last_block_core_height
            )).into()));
        }

        // Cleanup block cache before we execute a new proposal
        let mut drive_cache = self.drive.cache.write().unwrap();
        drive_cache.cached_contracts.clear_block_cache();
        drop(drive_cache);

        // destructure the block proposal
        let block_proposal::v0::BlockProposal {
            core_chain_locked_height,
            proposed_app_version,
            proposer_pro_tx_hash,
            validator_set_quorum_hash,
            raw_state_transitions,
            ..
        } = block_proposal;

        let block_info = block_state_info.to_block_info(
            Epoch::new(epoch_info.current_epoch_index())
                .expect("current epoch index should be in range"),
        );

        // Update the masternode list and create masternode identities and also update the active quorums
        self.update_core_info(
            Some(&state),
            &mut block_platform_state,
            core_chain_locked_height,
            false,
            &block_info,
            transaction,
            platform_version,
        )?;
        drop(state);

        // Update the validator proposed app version
        self.drive
            .update_validator_proposed_app_version(
                proposer_pro_tx_hash,
                proposed_app_version as u32,
                Some(transaction),
                &platform_version.drive,
            )
            .map_err(|e| {
                Error::Execution(ExecutionError::UpdateValidatorProposedAppVersionError(e))
            })?; // This is a system error

        let mut block_execution_context = block_execution_context::v0::BlockExecutionContextV0 {
            block_state_info: block_state_info.into(),
            epoch_info: epoch_info.clone(),
            hpmn_count: hpmn_list_len as u32,
            withdrawal_transactions: BTreeMap::new(),
            block_platform_state,
            proposer_results: None,
        };

        // Determine a new protocol version if enough proposers voted
        if block_execution_context
            .epoch_info
            .is_epoch_change_but_not_genesis()
        {
            tracing::info!(
                epoch_index = block_execution_context.epoch_info.current_epoch_index(),
                "epoch change occurring from epoch {} to epoch {}",
                block_execution_context
                    .epoch_info
                    .previous_epoch_index()
                    .expect("must be set since we aren't on genesis"),
                block_execution_context.epoch_info.current_epoch_index(),
            );

            if block_execution_context
                .block_platform_state
                .current_protocol_version_in_consensus()
                == block_execution_context
                    .block_platform_state
                    .next_epoch_protocol_version()
            {
                tracing::trace!(
                    epoch_index = block_execution_context.epoch_info.current_epoch_index(),
                    "protocol version remains the same {}",
                    block_execution_context
                        .block_platform_state
                        .current_protocol_version_in_consensus(),
                );
            } else {
                tracing::info!(
                    epoch_index = block_execution_context.epoch_info.current_epoch_index(),
                    "protocol version changed from {} to {}",
                    block_execution_context
                        .block_platform_state
                        .current_protocol_version_in_consensus(),
                    block_execution_context
                        .block_platform_state
                        .next_epoch_protocol_version(),
                );
            }

            // Set current protocol version to the version from upcoming epoch
            block_execution_context
                .block_platform_state
                .set_current_protocol_version_in_consensus(
                    block_execution_context
                        .block_platform_state
                        .next_epoch_protocol_version(),
                );

            // Determine new protocol version based on votes for the next epoch
            let maybe_new_protocol_version = self.check_for_desired_protocol_upgrade(
                block_execution_context.hpmn_count,
                block_execution_context
                    .block_platform_state
                    .current_protocol_version_in_consensus(),
                transaction,
            )?;

            if let Some(new_protocol_version) = maybe_new_protocol_version {
                block_execution_context
                    .block_platform_state
                    .set_next_epoch_protocol_version(new_protocol_version);
            } else {
                block_execution_context
                    .block_platform_state
                    .set_next_epoch_protocol_version(
                        block_execution_context
                            .block_platform_state
                            .current_protocol_version_in_consensus(),
                    );
            }
        }

        let mut block_execution_context: BlockExecutionContext = block_execution_context.into();

        // >>>>>> Withdrawal Status Update <<<<<<<
        // Only update the broadcasted withdrawal statuses if the core chain lock height has
        // changed. If it hasn't changed there should be no way a status could update

        if block_execution_context
            .block_state_info()
            .core_chain_locked_height()
            != last_block_core_height
        {
            self.update_broadcasted_withdrawal_transaction_statuses(
                &block_execution_context,
                transaction,
                platform_version,
            )?;
        }

        // This takes withdrawals from the transaction queue
        let unsigned_withdrawal_transaction_bytes = self
            .fetch_and_prepare_unsigned_withdrawal_transactions(
                validator_set_quorum_hash,
                &block_execution_context,
                transaction,
                platform_version,
            )?;

        // Set the withdrawal transactions that were done in the previous block
        block_execution_context.set_withdrawal_transactions(
            unsigned_withdrawal_transaction_bytes
                .into_iter()
                .map(|withdrawal_transaction| {
                    (
                        Txid::hash(withdrawal_transaction.as_slice()),
                        withdrawal_transaction,
                    )
                })
                .collect(),
        );

        let (block_fees, state_transition_results) = self.process_raw_state_transitions(
            raw_state_transitions,
            block_execution_context.block_platform_state(),
            &block_info,
            transaction,
            platform_version,
        )?;

        let mut block_execution_context: BlockExecutionContext = block_execution_context;

        self.pool_withdrawals_into_transactions_queue(
            &block_execution_context,
            transaction,
            platform_version,
        )?;

        // while we have the state transitions executed, we now need to process the block fees

        let block_fees_v0: BlockFeesV0 = block_fees.into();

        // Process fees
        let _processed_block_fees = self.process_block_fees(
            block_execution_context.block_state_info(),
            &epoch_info,
            block_fees_v0.into(),
            transaction,
            platform_version,
        )?;

        let root_hash = self
            .drive
            .grove
            .root_hash(Some(transaction))
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?; //GroveDb errors are system errors

        block_execution_context
            .block_state_info_mut()
            .set_app_hash(Some(root_hash));

        let state = self.state.read().unwrap();
        let validator_set_update =
            self.validator_set_update(&state, &mut block_execution_context, platform_version)?;

        self.block_execution_context
            .write()
            .unwrap()
            .replace(block_execution_context);

        Ok(ValidationResult::new_with_data(
            block_execution_outcome::v0::BlockExecutionOutcome {
                app_hash: root_hash,
                state_transition_results,
                validator_set_update,
                protocol_version: platform_version.protocol_version,
            },
        ))
    }
}
