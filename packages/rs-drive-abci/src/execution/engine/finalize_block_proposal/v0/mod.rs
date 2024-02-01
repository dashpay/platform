use dpp::block::epoch::Epoch;

use dpp::validation::SimpleValidationResult;

use drive::grovedb::Transaction;

use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::version::PlatformVersion;

use tenderdash_abci::{
    proto::{serializers::timestamp::ToMilis, types::BlockId as ProtoBlockId},
    signatures::Hashable,
};

use crate::abci::AbciError;
use crate::error::execution::ExecutionError;

use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_state_info::v0::{
    BlockStateInfoV0Getters, BlockStateInfoV0Methods,
};

use crate::platform_types::block_execution_outcome;
use crate::platform_types::cleaned_abci_messages::cleaned_block::v0::CleanedBlock;
use crate::platform_types::cleaned_abci_messages::finalized_block_cleaned_request::v0::FinalizeBlockCleanedRequest;

use crate::platform_types::commit::Commit;
use crate::platform_types::epoch_info::v0::EpochInfoV0Getters;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
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
    pub(super) fn finalize_block_proposal_v0(
        &self,
        request_finalize_block: FinalizeBlockCleanedRequest,
        transaction: &Transaction,
        _platform_version: &PlatformVersion,
    ) -> Result<block_execution_outcome::v0::BlockFinalizationOutcome, Error> {
        let mut validation_result = SimpleValidationResult::<AbciError>::new_with_errors(vec![]);

        // Retrieve block execution context before we do anything at all
        let guarded_block_execution_context = self.block_execution_context.read().unwrap();
        let block_execution_context =
            guarded_block_execution_context
                .as_ref()
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "block execution context must be set in block begin handler for finalize block proposal",
                )))?;

        let block_state_info = block_execution_context.block_state_info();
        let epoch_info = block_execution_context.epoch_info();
        let block_platform_state = block_execution_context.block_platform_state();

        let current_protocol_version = block_platform_state.current_protocol_version_in_consensus();

        let platform_version = PlatformVersion::get(current_protocol_version)?;

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

        let block_id_hash = Into::<ProtoBlockId>::into(block_id.clone())
            .calculate_msg_hash(&self.config.abci.chain_id, height as i64, round as i32)
            .map_err(AbciError::from)?
            .try_into()
            .expect("invalid sha256 length");

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
                hex::encode(hash),
                block_header.core_chain_locked_height,
                block_state_info.height(),
                block_state_info.round(),
                block_state_info.block_hash().map(hex::encode).unwrap_or("None".to_string()),
                block_state_info.core_chain_locked_height()
            )));
            return Ok(validation_result.into());
        }

        let state_cache = self.state.read().unwrap();
        let current_quorum_hash = state_cache.current_validator_set_quorum_hash().into();
        if current_quorum_hash != commit_info.quorum_hash {
            validation_result.add_error(AbciError::WrongFinalizeBlockReceived(format!(
                "received a block for h: {} r: {} with validator set quorum hash {} expected current validator set quorum hash is {}",
                height, round, hex::encode(commit_info.quorum_hash), hex::encode(block_platform_state.current_validator_set_quorum_hash())
            )));
            return Ok(validation_result.into());
        }

        let quorum_public_key = &state_cache.current_validator_set()?.threshold_public_key();

        // In production this will always be true
        if self
            .config
            .testing_configs
            .block_commit_signature_verification
        {
            // Verify commit

            let quorum_type = self.config.validator_set_quorum_type();
            let commit = Commit::new_from_cleaned(
                commit_info.clone(),
                block_id,
                height,
                quorum_type,
                &block_header.chain_id,
                platform_version,
            )?;
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
            self.drive
                .set_genesis_time(block_state_info.block_time_ms());
        }

        let mut to_commit_block_info: BlockInfo = block_state_info.to_block_info(
            Epoch::new(epoch_info.current_epoch_index())
                .expect("current epoch info should be in range"),
        );

        // we need to add the block time
        to_commit_block_info.time_ms = block_header.time.to_milis();

        to_commit_block_info.core_height = block_header.core_chain_locked_height;

        // // Finalize withdrawal processing
        // our_withdrawals.finalize(Some(transaction), &self.drive, &to_commit_block_info)?;

        // At the end we update the state cache

        drop(guarded_block_execution_context);

        let extended_block_info = ExtendedBlockInfoV0 {
            basic_info: to_commit_block_info,
            app_hash: block_header.app_hash,
            quorum_hash: current_quorum_hash,
            block_id_hash,
            signature: commit_info.block_signature,
            round,
        }
        .into();

        self.update_state_cache(extended_block_info, transaction, platform_version)?;

        self.update_drive_cache(platform_version)?;

        // Gather some metrics
        crate::metrics::abci_last_block_time(block_header.time.seconds as u64);
        crate::metrics::abci_last_platform_height(height);
        crate::metrics::abci_last_finalized_round(round);

        Ok(validation_result.into())
    }
}
