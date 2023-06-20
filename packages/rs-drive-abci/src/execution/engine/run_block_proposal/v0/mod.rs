use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::Txid;

use dpp::block::epoch::Epoch;

use dpp::validation::ValidationResult;
use drive::error::Error::GroveDB;

use drive::grovedb::Transaction;
use std::collections::BTreeMap;

use crate::abci::AbciError;
use crate::error::execution::ExecutionError;

use crate::error::Error;
use crate::execution::types::{block_execution_context, block_state_info};

use crate::platform_types::block_execution_outcome;
use crate::platform_types::block_proposal;
use crate::platform_types::epoch::v0::EpochInfo;
use crate::platform_types::platform::Platform;
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
        transaction: &Transaction,
    ) -> Result<ValidationResult<block_execution_outcome::v0::BlockExecutionOutcome, Error>, Error>
    {
        // Start by getting information from the state
        let state = self.state.read().unwrap();

        let last_block_time_ms = state.last_block_time_ms();
        let last_block_height =
            state.known_height_or(self.config.abci.genesis_height.saturating_sub(1));
        let last_block_core_height =
            state.known_core_height_or(self.config.abci.genesis_core_height);
        let hpmn_list_len = state.hpmn_list_len();
        let _quorum_hash = state.current_validator_set_quorum_hash;

        let mut block_platform_state = state.clone();

        // Init block execution context
        let block_state_info = block_state_info::v0::BlockStateInfo::from_block_proposal(
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

        // destructure the block proposal
        let block_proposal::v0::BlockProposal {
            consensus_versions: _,
            block_hash: _,
            height,
            round: _,
            core_chain_locked_height,
            proposed_app_version,
            proposer_pro_tx_hash,
            validator_set_quorum_hash,
            block_time_ms,
            raw_state_transitions,
        } = block_proposal;
        // todo: verify that we support the consensus versions
        // We start by getting the epoch we are in
        let genesis_time_ms = self.get_genesis_time_v0(height, block_time_ms, transaction)?;

        let epoch_info =
            EpochInfo::from_genesis_time_and_block_info(genesis_time_ms, &block_state_info)?;

        let block_info = block_state_info.to_block_info(
            Epoch::new(epoch_info.current_epoch_index)
                .expect("current epoch index should be in range"),
        );

        // Update the masternode list and create masternode identities and also update the active quorums
        self.update_core_info_v0(
            Some(&state),
            &mut block_platform_state,
            core_chain_locked_height,
            false,
            &block_info,
            transaction,
        )?;
        drop(state);

        // Update the validator proposed app version
        self.drive
            .update_validator_proposed_app_version(
                proposer_pro_tx_hash,
                proposed_app_version as u32,
                Some(transaction),
            )
            .map_err(|e| {
                Error::Execution(ExecutionError::UpdateValidatorProposedAppVersionError(e))
            })?; // This is a system error

        let mut block_execution_context = block_execution_context::v0::BlockExecutionContext {
            block_state_info,
            epoch_info: epoch_info.clone(),
            hpmn_count: hpmn_list_len as u32,
            withdrawal_transactions: BTreeMap::new(),
            block_platform_state,
            proposer_results: None,
        };

        // If last synced Core block height is not set instead of scanning
        // number of blocks for asset unlock transactions scan only one
        // on Core chain locked height by setting last_synced_core_height to the same value
        // FIXME: re-enable and implement
        // let last_synced_core_height = if request.last_synced_core_height == 0 {
        //     block_execution_context.block_info.core_chain_locked_height
        // } else {
        //     request.last_synced_core_height
        // };

        // Determine a new protocol version if enough proposers voted
        if block_execution_context
            .epoch_info
            .is_epoch_change_but_not_genesis()
        {
            // Set current protocol version to the version from upcoming epoch
            block_execution_context
                .block_platform_state
                .current_protocol_version_in_consensus = block_execution_context
                .block_platform_state
                .next_epoch_protocol_version;

            // Determine new protocol version based on votes for the next epoch
            let maybe_new_protocol_version = self.check_for_desired_protocol_upgrade(
                block_execution_context.hpmn_count,
                block_execution_context
                    .block_platform_state
                    .current_protocol_version_in_consensus,
                transaction,
            )?;
            if let Some(new_protocol_version) = maybe_new_protocol_version {
                block_execution_context
                    .block_platform_state
                    .next_epoch_protocol_version = new_protocol_version;
            } else {
                block_execution_context
                    .block_platform_state
                    .next_epoch_protocol_version = block_execution_context
                    .block_platform_state
                    .current_protocol_version_in_consensus;
            }
        }

        let last_synced_core_height = block_execution_context
            .block_state_info
            .core_chain_locked_height;

        self.update_broadcasted_withdrawal_transaction_statuses_v0(
            last_synced_core_height,
            &block_execution_context,
            transaction,
        )?;

        // This takes withdrawals from the transaction queue
        let unsigned_withdrawal_transaction_bytes = self
            .fetch_and_prepare_unsigned_withdrawal_transactions_v0(
                validator_set_quorum_hash,
                &block_execution_context,
                transaction,
            )?;

        // Set the withdrawal transactions that were done in the previous block
        block_execution_context.withdrawal_transactions = unsigned_withdrawal_transaction_bytes
            .into_iter()
            .map(|withdrawal_transaction| {
                (
                    Txid::hash(withdrawal_transaction.as_slice()),
                    withdrawal_transaction,
                )
            })
            .collect();

        let (block_fees, tx_results) = self.process_raw_state_transitions_v0(
            raw_state_transitions,
            &block_execution_context.block_platform_state,
            &block_info,
            transaction,
        )?;

        self.pool_withdrawals_into_transactions_queue_v0(&block_execution_context, transaction)?;

        // while we have the state transitions executed, we now need to process the block fees

        // Process fees
        let _processed_block_fees = self.process_block_fees_v0(
            &block_execution_context.block_state_info,
            &epoch_info,
            block_fees.into(),
            transaction,
        )?;

        let root_hash = self
            .drive
            .grove
            .root_hash(Some(transaction))
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?; //GroveDb errors are system errors

        block_execution_context.block_state_info.app_hash = Some(root_hash);

        let state = self.state.read().unwrap();
        let validator_set_update =
            self.validator_set_update_v0(&state, &mut block_execution_context)?;

        self.block_execution_context
            .write()
            .unwrap()
            .replace(block_execution_context);

        Ok(ValidationResult::new_with_data(
            block_execution_outcome::v0::BlockExecutionOutcome {
                app_hash: root_hash,
                tx_results,
                validator_set_update,
            },
        ))
    }
}
