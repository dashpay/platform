use dashcore_rpc::dashcore::hashes::{hex::ToHex, Hash};
use dashcore_rpc::dashcore::Txid;
use dpp::block::block_info::{BlockInfo, ExtendedBlockInfo};
use dpp::block::epoch::Epoch;
use dpp::bls_signatures;

use dpp::validation::{SimpleValidationResult, ValidationResult};
use drive::error::Error::GroveDB;

use drive::grovedb::Transaction;
use std::collections::BTreeMap;

use tenderdash_abci::proto::abci::{ExecTxResult, ValidatorSetUpdate};
use tenderdash_abci::proto::serializers::timestamp::ToMilis;

use crate::abci::AbciError;
use crate::error::execution::ExecutionError;

use crate::error::Error;
use crate::execution::types::{block_execution_context, block_state_info};
use crate::platform::block_proposal;
use crate::platform::cleaned_abci_messages::cleaned_block::v0::CleanedBlock;
use crate::platform::cleaned_abci_messages::finalized_block_cleaned_request::v0::FinalizeBlockCleanedRequest;
use crate::platform::epoch::v0::EpochInfo;
use crate::platform::{block_execution_outcome, Platform};
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
    pub fn run_block_proposal(
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
        let genesis_time_ms = self.get_genesis_time(height, block_time_ms, transaction)?;

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

    /// Finalizes the block proposal by first validating it and then committing it to the state.
    ///
    /// This function first retrieves the block execution context and decomposes the request. It then checks
    /// if the received block matches the expected block information (height, round, hash, etc.). If everything
    /// matches, the function verifies the commit signature (if enabled) and the vote extensions. If all checks
    /// pass, the block is committed to the state.
    ///
    /// # Arguments
    ///
    /// * `request_finalize_block` - A `FinalizeBlockCleanedRequest` object containing the block proposal data.
    /// * `transaction` - A reference to a `Transaction` object.
    ///
    /// # Returns
    ///
    /// * `Result<BlockFinalizationOutcome, Error>` - If the block proposal passes all checks and is committed
    ///   to the state, it returns a `BlockFinalizationOutcome`. If any check fails, it returns an `Error`.
    ///
    pub fn finalize_block_proposal(
        &self,
        request_finalize_block: FinalizeBlockCleanedRequest,
        transaction: &Transaction,
    ) -> Result<BlockFinalizationOutcome, Error> {
        let mut validation_result = SimpleValidationResult::<AbciError>::new_with_errors(vec![]);

        // Retrieve block execution context before we do anything at all
        let guarded_block_execution_context = self.block_execution_context.read().unwrap();
        let block_execution_context =
            guarded_block_execution_context
                .as_ref()
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "block execution context must be set in block begin handler for finalize block proposal",
                )))?;

        let BlockExecutionContext {
            block_state_info,
            epoch_info,
            block_platform_state,
            ..
        } = &block_execution_context;

        // Let's decompose the request
        let FinalizeBlockCleanedRequest {
            commit: commit_info,
            misbehavior: _,
            hash,
            height,
            round,
            block,
            block_id,
        } = request_finalize_block;

        let CleanedBlock {
            header: block_header,
            data: _,
            evidence: _,
            last_commit: _,
            core_chain_lock: _,
        } = block;

        //// Verification that commit is for our current executed block
        // When receiving the finalized block, we need to make sure that info matches our current block

        // First let's check the basics, height, round and hash
        if !block_state_info.matches_expected_block_info(
            height,
            round,
            block_header.core_chain_locked_height,
            block_header.proposer_pro_tx_hash,
            hash,
        )? {
            // we are on the wrong height or round
            validation_result.add_error(AbciError::WrongFinalizeBlockReceived(format!(
                "received a block for h: {} r: {}, block hash: {}, core height: {}, expected h: {} r: {}, block hash: {}, core height: {}",
                height,
                round,
                hash.to_hex(),
                block_header.core_chain_locked_height,
                block_state_info.height,
                block_state_info.round,
                block_state_info.block_hash.map(|a| a.to_hex()).unwrap_or("None".to_string()),
                block_state_info.core_chain_locked_height
            )));
            return Ok(validation_result.into());
        }

        let state_cache = self.state.read().unwrap();
        let current_quorum_hash = state_cache.current_validator_set_quorum_hash.into_inner();
        if current_quorum_hash != commit_info.quorum_hash {
            validation_result.add_error(AbciError::WrongFinalizeBlockReceived(format!(
                "received a block for h: {} r: {} with validator set quorum hash {} expected current validator set quorum hash is {}",
                height, round, hex::encode(commit_info.quorum_hash), hex::encode(block_platform_state.current_validator_set_quorum_hash)
            )));
            return Ok(validation_result.into());
        }

        let quorum_public_key = &state_cache.current_validator_set()?.threshold_public_key;

        // In production this will always be true
        if self
            .config
            .testing_configs
            .block_commit_signature_verification
        {
            // Verify commit

            let quorum_type = self.config.quorum_type();
            let commit = Commit::new_from_cleaned(
                commit_info.clone(),
                block_id,
                height,
                quorum_type,
                &block_header.chain_id,
            );
            let validation_result =
                commit.verify_signature(&commit_info.block_signature, quorum_public_key);

            if !validation_result.is_valid() {
                return Ok(validation_result.into());
            }
        }
        drop(state_cache);

        // Verify vote extensions
        // let received_withdrawals = WithdrawalTxs::from(&commit.threshold_vote_extensions);
        // let our_withdrawals = WithdrawalTxs::load(Some(transaction), &self.drive)
        //     .map_err(|e| AbciError::WithdrawalTransactionsDBLoadError(e.to_string()))?;
        //todo: reenable check
        //
        // if let Err(e) = self.check_withdrawals(
        //     &received_withdrawals,
        //     &our_withdrawals,
        //     Some(quorum_public_key),
        // ) {
        //     validation_result.add_error(e);
        //     return Ok(validation_result.into());
        // }

        // Next let's check that the hash received is the same as the hash we expect

        if height == self.config.abci.genesis_height {
            self.drive.set_genesis_time(block_state_info.block_time_ms);
        }

        let mut to_commit_block_info: BlockInfo = block_state_info.to_block_info(
            Epoch::new(epoch_info.current_epoch_index)
                .expect("current epoch info should be in range"),
        );

        // we need to add the block time
        to_commit_block_info.time_ms = block_header.time.to_milis();

        to_commit_block_info.core_height = block_header.core_chain_locked_height;

        // // Finalize withdrawal processing
        // our_withdrawals.finalize(Some(transaction), &self.drive, &to_commit_block_info)?;

        // At the end we update the state cache

        drop(guarded_block_execution_context);

        let extended_block_info = ExtendedBlockInfo {
            basic_info: to_commit_block_info,
            app_hash: block_header.app_hash,
            quorum_hash: current_quorum_hash,
            signature: commit_info.block_signature,
            round,
        };

        self.update_state_cache_v0(extended_block_info, transaction)?;

        let mut drive_cache = self.drive.cache.write().unwrap();

        drive_cache.cached_contracts.clear_block_cache();

        // Gather some metrics
        crate::metrics::abci_last_block_time(block_header.time.seconds as u64);
        crate::metrics::abci_last_platform_height(height);
        crate::metrics::abci_last_finalized_round(round);

        Ok(validation_result.into())
    }
}
