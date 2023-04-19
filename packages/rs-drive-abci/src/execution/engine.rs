use bls_signatures;
use dashcore::hashes::Hash;
use dashcore::{QuorumHash, Txid};

use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::consensus::basic::identity::IdentityInsufficientBalanceError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::StateTransition;
use dpp::validation::{
    ConsensusValidationResult, SimpleConsensusValidationResult, SimpleValidationResult,
    ValidationResult,
};
use drive::error::Error::GroveDB;
use drive::fee::result::FeeResult;
use drive::grovedb::{Transaction, TransactionArg};
use std::collections::BTreeMap;

use tenderdash_abci::proto::abci::ExecTxResult;
use tenderdash_abci::proto::serializers::timestamp::ToMilis;

use crate::abci::commit::Commit;
use crate::abci::withdrawal::WithdrawalTxs;
use crate::abci::AbciError;
use crate::block::{BlockExecutionContext, BlockStateInfo};
use crate::error::execution::ExecutionError;

use crate::error::Error;
use crate::execution::block_proposal::BlockProposal;
use crate::execution::execution_event::ExecutionResult::{
    ConsensusExecutionError, SuccessfulFreeExecution, SuccessfulPaidExecution,
};
use crate::execution::execution_event::{ExecutionEvent, ExecutionResult};
use crate::execution::fee_pools::epoch::EpochInfo;
use crate::execution::finalize_block_cleaned_request::{CleanedBlock, FinalizeBlockCleanedRequest};
use crate::platform::{Platform, PlatformRef};
use crate::rpc::core::CoreRPCLike;

use crate::validation::state_transition::process_state_transition;

/// The outcome of the block execution, either by prepare proposal, or process proposal
#[derive(Clone)]
pub struct BlockExecutionOutcome {
    /// The app hash, also known as the commit hash, this is the root hash of grovedb
    /// after the block has been executed
    pub app_hash: [u8; 32],
    /// The results of the execution of each transaction
    pub tx_results: Vec<ExecTxResult>,
}

/// The outcome of the finalization of the block
pub struct BlockFinalizationOutcome {
    /// The validation result of the finalization of the block.
    /// Errors here can happen if the block that we receive to be finalized isn't actually
    /// the one we expect, this could be a replay attack or some other kind of attack.
    pub validation_result: SimpleValidationResult<AbciError>,
}

impl From<SimpleValidationResult<AbciError>> for BlockFinalizationOutcome {
    fn from(validation_result: SimpleValidationResult<AbciError>) -> Self {
        BlockFinalizationOutcome { validation_result }
    }
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(crate) fn validate_fees_of_event(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<FeeResult>, Error> {
        match event {
            ExecutionEvent::PaidFromAssetLockDriveEvent {
                identity,
                operations,
            }
            | ExecutionEvent::PaidDriveEvent {
                identity,
                operations,
            } => {
                let balance = identity.balance.ok_or(Error::Execution(
                    ExecutionError::CorruptedCodeExecution("partial identity info with no balance"),
                ))?;
                let estimated_fee_result = self
                    .drive
                    .apply_drive_operations(operations.clone(), false, block_info, transaction)
                    .map_err(Error::Drive)?;

                // TODO: Should take into account refunds as well
                if balance >= estimated_fee_result.total_base_fee() {
                    Ok(ConsensusValidationResult::new_with_data(
                        estimated_fee_result,
                    ))
                } else {
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![ConsensusError::IdentityInsufficientBalanceError(
                            IdentityInsufficientBalanceError {
                                identity_id: identity.id,
                                balance,
                            },
                        )],
                    ))
                }
            }
            ExecutionEvent::FreeDriveEvent { .. } => Ok(ConsensusValidationResult::new_with_data(
                FeeResult::default(),
            )),
        }
    }

    pub(crate) fn execute_event(
        &self,
        event: ExecutionEvent,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<ExecutionResult, Error> {
        //todo: we need to split out errors
        //  between failed execution and internal errors
        let validation_result =
            self.validate_fees_of_event(&event, block_info, Some(transaction))?;
        match event {
            ExecutionEvent::PaidFromAssetLockDriveEvent {
                identity,
                operations,
            }
            | ExecutionEvent::PaidDriveEvent {
                identity,
                operations,
            } => {
                if validation_result.is_valid_with_data() {
                    //todo: make this into an atomic event with partial batches
                    let individual_fee_result = self
                        .drive
                        .apply_drive_operations(operations, true, block_info, Some(transaction))
                        .map_err(Error::Drive)?;

                    let balance_change =
                        individual_fee_result.into_balance_change(identity.id.to_buffer());

                    let outcome = self.drive.apply_balance_change_from_fee_to_identity(
                        balance_change,
                        Some(transaction),
                    )?;

                    // println!("State transition fees {:#?}", outcome.actual_fee_paid);
                    //
                    // println!(
                    //     "Identity balance {:?} changed {:#?}",
                    //     identity.balance,
                    //     balance_change.change()
                    // );

                    Ok(SuccessfulPaidExecution(
                        validation_result.into_data()?,
                        outcome.actual_fee_paid,
                    ))
                } else {
                    Ok(ConsensusExecutionError(
                        SimpleConsensusValidationResult::new_with_errors(validation_result.errors),
                    ))
                }
            }
            ExecutionEvent::FreeDriveEvent { operations } => {
                self.drive
                    .apply_drive_operations(operations, true, block_info, Some(transaction))
                    .map_err(Error::Drive)?;
                Ok(SuccessfulFreeExecution)
            }
        }
    }

    pub(crate) fn process_raw_state_transitions(
        &self,
        raw_state_transitions: &Vec<Vec<u8>>,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<(FeeResult, Vec<ExecTxResult>), Error> {
        let state_transitions = StateTransition::deserialize_many(raw_state_transitions)?;
        let mut aggregate_fee_result = FeeResult::default();
        let state_read_guard = self.state.read().unwrap();
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: &state_read_guard,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };
        let exec_tx_results = state_transitions
            .into_iter()
            .map(|state_transition| {
                let state_transition_execution_event =
                    process_state_transition(&platform_ref, state_transition, Some(transaction))?;

                let execution_result = if state_transition_execution_event.is_valid() {
                    let execution_event = state_transition_execution_event.into_data()?;
                    self.execute_event(execution_event, block_info, transaction)?
                } else {
                    ConsensusExecutionError(SimpleConsensusValidationResult::new_with_errors(
                        state_transition_execution_event.errors,
                    ))
                };
                if let SuccessfulPaidExecution(_, fee_result) = &execution_result {
                    aggregate_fee_result.checked_add_assign(fee_result.clone())?;
                }

                Ok(execution_result.into())
            })
            .collect::<Result<Vec<ExecTxResult>, Error>>()?;
        Ok((aggregate_fee_result, exec_tx_results))
    }

    // /// Update of the masternode identities
    // pub fn update_masternode_identities(
    //     &self,
    //     previous_core_height: u32,
    //     current_core_height: u32,
    // ) -> Result<(), Error> {
    //     if previous_core_height != current_core_height {
    //         //todo:
    //         // self.drive.fetch_full_identity()
    //         // self.drive.add_new_non_unique_keys_to_identity()
    //     }
    //     Ok(())
    // }

    /// Run a block proposal, either from process proposal, or prepare proposal
    /// Errors returned in the validation result are consensus errors, any error here means that
    /// the block should be rejected
    /// Errors returned in the result are critical system errors
    pub fn run_block_proposal(
        &self,
        block_proposal: BlockProposal,
        transaction: &Transaction,
    ) -> Result<ValidationResult<BlockExecutionOutcome, Error>, Error> {
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
        drop(state);

        // Init block execution context
        let block_state_info =
            BlockStateInfo::from_block_proposal(&block_proposal, last_block_time_ms);

        // First let's check that this is the follower to a previous block
        if !block_state_info.next_block_to(last_block_height, last_block_core_height)? {
            // we are on the wrong height or round
            return Ok(ValidationResult::new_with_error(AbciError::WrongFinalizeBlockReceived(format!(
                "received a block proposal for height: {} core height: {}, current height: {} core height: {}",
                block_state_info.height, block_state_info.core_chain_locked_height, last_block_height, last_block_core_height
            )).into()));
        }

        // destructure the block proposal
        let BlockProposal {
            consensus_versions: _,
            block_hash: _,
            height,
            round: _,
            core_chain_locked_height: _,
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

        // Update the active quorums
        self.update_quorum_info(
            &mut block_platform_state,
            block_proposal.core_chain_locked_height,
        )?;

        // Update the masternode list and create masternode identities
        self.update_masternode_list(
            &mut block_platform_state,
            block_proposal.core_chain_locked_height,
            false,
            &block_info,
            transaction,
        )?;

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

        let mut block_execution_context = BlockExecutionContext {
            block_state_info,
            epoch_info: epoch_info.clone(),
            hpmn_count: hpmn_list_len as u32,
            withdrawal_transactions: BTreeMap::new(),
            block_platform_state,
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
        let last_synced_core_height = block_execution_context
            .block_state_info
            .core_chain_locked_height;

        self.update_broadcasted_withdrawal_transaction_statuses(
            last_synced_core_height,
            &block_execution_context,
            transaction,
        )?;

        // This takes withdrawals from the transaction queue
        let unsigned_withdrawal_transaction_bytes = self
            .fetch_and_prepare_unsigned_withdrawal_transactions(
                validator_set_quorum_hash,
                &block_execution_context,
                transaction,
            )?;

        // Set the withdrawal transactions
        block_execution_context.withdrawal_transactions = unsigned_withdrawal_transaction_bytes
            .into_iter()
            .map(|withdrawal_transaction| {
                (
                    Txid::hash(withdrawal_transaction.as_slice()),
                    withdrawal_transaction,
                )
            })
            .collect();

        let (block_fees, tx_results) =
            self.process_raw_state_transitions(raw_state_transitions, &block_info, transaction)?;

        self.pool_withdrawals_into_transactions_queue(&block_execution_context, transaction)?;

        // while we have the state transitions executed, we now need to process the block fees

        // Process fees
        let _process_block_fees_outcome = self.process_block_fees(
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

        block_execution_context.block_state_info.commit_hash = Some(root_hash);

        self.block_execution_context
            .write()
            .unwrap()
            .replace(block_execution_context);

        Ok(ValidationResult::new_with_data(BlockExecutionOutcome {
            app_hash: root_hash,
            tx_results,
        }))
    }

    // TODO: remove function from here
    fn store_ephemeral_data(
        &self,
        block_info: &BlockInfo,
        quorum_hash: &QuorumHash,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        // we need to serialize the block info
        let serialized_block_info = block_info.serialize()?;

        // next we need to store this data in groveb
        self.drive
            .grove
            .put_aux(
                b"saved_state",
                &serialized_block_info,
                None,
                Some(transaction),
            )
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        self.drive
            .grove
            .put_aux(
                b"saved_quorum_hash",
                &quorum_hash.into_inner(),
                None,
                Some(transaction),
            )
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        Ok(())
    }

    /// Update the current quorums if the core_height changes
    pub fn update_state_cache_and_quorums(
        &self,
        block_info: BlockInfo,
        updated_validator_hash: QuorumHash,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        let mut block_execution_context = self.block_execution_context.write().unwrap();

        let block_execution_context = block_execution_context.take().ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution("there should be a block execution context"),
        ))?;

        let mut state_cache = self.state.write().unwrap();

        *state_cache = block_execution_context.block_platform_state;

        // Determine a new protocol version if enough proposers voted
        if block_execution_context
            .epoch_info
            .is_epoch_change_but_not_genesis()
        {
            // Set current protocol version to the version from upcoming epoch
            state_cache.current_protocol_version_in_consensus =
                state_cache.next_epoch_protocol_version;

            // Determine new protocol version based on votes for the next epoch
            let maybe_new_protocol_version = self.check_for_desired_protocol_upgrade(
                block_execution_context.hpmn_count,
                &state_cache,
                transaction,
            )?;
            if let Some(new_protocol_version) = maybe_new_protocol_version {
                state_cache.next_epoch_protocol_version = new_protocol_version;
            } else {
                state_cache.next_epoch_protocol_version =
                    state_cache.current_protocol_version_in_consensus;
            }
        }

        state_cache.current_validator_set_quorum_hash = updated_validator_hash;

        state_cache.last_committed_block_info = Some(block_info.clone());

        // Persist ephemeral data
        self.store_ephemeral_data(&block_info, &updated_validator_hash, transaction)?;

        Ok(())
    }

    /// check if received withdrawal transactions are correct and match our withdrawal txs
    pub fn check_withdrawals(
        &self,
        received_withdrawals: &WithdrawalTxs,
        our_withdrawals: &WithdrawalTxs,
        height: u64,
        round: u32,
        verify_with_validator_public_key: Option<&bls_signatures::PublicKey>,
        quorum_hash: Option<&[u8]>,
    ) -> SimpleValidationResult<AbciError> {
        if received_withdrawals.ne(our_withdrawals) {
            return SimpleValidationResult::new_with_error(
                AbciError::VoteExtensionMismatchReceived {
                    got: received_withdrawals.to_string(),
                    expected: our_withdrawals.to_string(),
                },
            );
        }

        // we only verify if verify_with_validator_public_key exists
        if let Some(validator_public_key) = verify_with_validator_public_key {
            let quorum_hash = quorum_hash.expect("quorum hash is required to verify signature");
            let validation_result = received_withdrawals.verify_signatures(
                &self.config.abci.chain_id,
                self.config.quorum_type(),
                quorum_hash,
                height,
                round,
                validator_public_key,
            );

            if validation_result.is_valid() {
                SimpleValidationResult::default()
            } else {
                SimpleValidationResult::new_with_error(
                    validation_result
                        .errors
                        .into_iter()
                        .next()
                        .expect("expected an error"),
                )
            }
        } else {
            SimpleValidationResult::default()
        }
    }

    // Retrieve quorum public key
    fn get_quorum_key(&self, quorum_hash: [u8; 32]) -> Result<bls_signatures::PublicKey, Error> {
        let public_key = self
            .core_rpc
            .get_quorum_info(
                self.config.quorum_type(),
                &QuorumHash::from_inner(quorum_hash),
                Some(false),
            )?
            .quorum_public_key;

        bls_signatures::PublicKey::from_bytes(public_key.as_slice())
            .map_err(|e| Error::Execution(ExecutionError::BlsErrorFromDashCoreResponse(e)))
    }

    /// Finalize the block, this first involves validating it, then if valid
    /// it is committed to the state
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
                    "block execution context must be set in block begin handler",
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
                "received a block for h: {} r: {}, expected h: {} r: {}",
                height, round, block_state_info.height, block_state_info.round
            )));
            return Ok(validation_result.into());
        }

        if block_platform_state
            .current_validator_set_quorum_hash
            .as_inner()
            != &commit_info.quorum_hash
        {
            validation_result.add_error(AbciError::WrongFinalizeBlockReceived(format!(
                "received a block for h: {} r: {} with validator set quorum hash {} expected current validator set quorum hash is {}",
                height, round, hex::encode(commit_info.quorum_hash), hex::encode(block_platform_state.current_validator_set_quorum_hash)
            )));
            return Ok(validation_result.into());
        }

        // In production this will always be true
        if self
            .config
            .testing_configs
            .block_commit_signature_verification
        {
            let quorum_public_key = self.get_quorum_key(commit_info.quorum_hash)?;

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
                commit.verify_signature(&commit_info.block_signature, &quorum_public_key);

            if !validation_result.is_valid() {
                return Ok(validation_result.into());
            }
        }

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
        self.update_state_cache_and_quorums(
            to_commit_block_info,
            QuorumHash::from_inner(block_header.next_validators_hash),
            transaction,
        )?;

        let mut drive_cache = self.drive.cache.write().unwrap();

        drive_cache.cached_contracts.clear_block_cache();

        Ok(validation_result.into())
    }

    /// Check a state transition to see if it should be added to mempool,
    /// This executes a few checks.
    /// It does validation on the state transition, and checks that the user is able to pay
    /// for the it. It can be wrong is rare cases, so the proposer needs to check transactions
    /// again before proposing his block.
    pub fn check_tx(
        &self,
        raw_tx: Vec<u8>,
    ) -> Result<ValidationResult<FeeResult, ConsensusError>, Error> {
        let state_transition =
            StateTransition::deserialize(raw_tx.as_slice()).map_err(Error::Protocol)?;
        let state_read_guard = self.state.read().unwrap();
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: &state_read_guard,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };
        let execution_event = process_state_transition(&platform_ref, state_transition, None)?;

        // We should run the execution event in dry run to see if we would have enough fees for the transaction

        // We need the approximate block info
        if let Some(block_info) = state_read_guard.last_committed_block_info.as_ref() {
            // We do not put the transaction, because this event happens outside of a block
            execution_event.and_then_borrowed_validation(|execution_event| {
                self.validate_fees_of_event(execution_event, block_info, None)
            })
        } else {
            execution_event.and_then_borrowed_validation(|execution_event| {
                self.validate_fees_of_event(execution_event, &BlockInfo::default(), None)
            })
        }
    }
}
