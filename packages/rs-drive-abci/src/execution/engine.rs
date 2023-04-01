use crate::abci::messages::{
    AfterFinalizeBlockRequest, BlockBeginRequest, BlockEndRequest, BlockFees,
};
use crate::block::{BlockExecutionContext, BlockStateInfo};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::block_proposal::BlockProposal;
use crate::execution::execution_event::ExecutionResult::{
    ConsensusExecutionError, SuccessfulFreeExecution, SuccessfulPaidExecution,
};
use crate::execution::execution_event::{ExecutionEvent, ExecutionResult};
use crate::execution::fee_pools::epoch::EpochInfo;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformState;
use crate::validation::state_transition::StateTransitionValidation;
use dpp::consensus::basic::identity::IdentityInsufficientBalanceError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::StateTransition;
use dpp::validation::{SimpleValidationResult, ValidationResult};
use drive::dpp::identity::PartialIdentity;
use drive::dpp::util::deserializer::ProtocolVersion;
use drive::drive::batch::DriveOperation;
use drive::drive::block_info::BlockInfo;
use drive::drive::Drive;
use drive::error::Error::GroveDB;
use drive::fee::result::FeeResult;
use drive::grovedb::{Transaction, TransactionArg};
use tenderdash_abci::proto::abci::{ExecTxResult, RequestPrepareProposal, ResponsePrepareProposal};
use tenderdash_abci::proto::serializers::timestamp::ToMilis;

pub struct BlockExecutionOutcome<'a> {
    block_execution_context: BlockExecutionContext<'a>,
    tx_results: Vec<ExecTxResult>,
}

impl<'a, C> Platform<'a, C>
where
    C: CoreRPCLike,
{
    pub(crate) fn validate_fees_of_event(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<ValidationResult<FeeResult>, Error> {
        match event {
            ExecutionEvent::PaidDriveEvent {
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
                    Ok(ValidationResult::new_with_data(estimated_fee_result))
                } else {
                    Ok(ValidationResult::new_with_data_and_errors(
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
            ExecutionEvent::FreeDriveEvent { operations } => {
                Ok(ValidationResult::new_with_data(FeeResult::default()))
            }
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
        match event {
            ExecutionEvent::PaidDriveEvent {
                identity,
                operations,
            } => {
                let validation_result =
                    self.validate_fees_of_event(&event, block_info, Some(&transaction))?;
                if validation_result.is_valid_with_data() {
                    let individual_fee_result = self
                        .drive
                        .apply_drive_operations(operations, true, block_info, Some(transaction))
                        .map_err(Error::Drive)?;

                    let balance_change =
                        individual_fee_result.into_balance_change(identity.id.to_buffer());

                    let outcome = self.drive.apply_balance_change_from_fee_to_identity(
                        balance_change.clone(),
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
                        SimpleValidationResult::new_with_errors(validation_result.errors),
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

    fn execute_events(
        &self,
        events: Vec<ExecutionEvent>,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<FeeResult, Error> {
        let mut total_fees = FeeResult::default();
        for event in events {
            self.execute_event(event, block_info, transaction)
        }
        Ok(total_fees)
    }

    // /// Execute a block with various state transitions
    // pub fn execute_block(
    //     &self,
    //     proposer: [u8; 32],
    //     proposed_version: ProtocolVersion,
    //     total_hpmns: u32,
    //     block_info: BlockInfo,
    //     state_transitions: Vec<ExecutionEvent>,
    // ) -> Result<(), Error> {
    //     let transaction = self.drive.grove.start_transaction();
    //     // Processing block
    //     let block_begin_request = BlockBeginRequest {
    //         block_height: block_info.height,
    //         block_time_ms: block_info.time_ms,
    //         previous_block_time_ms: self
    //             .state
    //             .read()
    //             .unwrap()
    //             .last_block_info
    //             .as_ref()
    //             .map(|block_info| block_info.time_ms),
    //         proposer_pro_tx_hash: proposer,
    //         proposed_app_version: proposed_version,
    //         validator_set_quorum_hash: Default::default(),
    //         last_synced_core_height: 1,
    //         core_chain_locked_height: 1,
    //         total_hpmns,
    //     };
    //
    //     // println!("Block #{}", block_info.height);
    //
    //     let _block_begin_response = self.block_begin(block_begin_request).unwrap_or_else(|e| {
    //         panic!(
    //             "should begin process block #{} at time #{} : {e}",
    //             block_info.height, block_info.time_ms
    //         )
    //     });
    //
    //     // println!("{:#?}", block_begin_response);
    //
    //     let total_fees = self.execute_events(state_transitions, &block_info, &transaction)?;
    //
    //     let fees = BlockFees::from_fee_result(total_fees);
    //
    //     let block_end_request = BlockEndRequest { fees };
    //
    //     let _block_end_response = self.block_end(block_end_request).unwrap_or_else(|e| {
    //         panic!(
    //             "engine should end process block #{} at time #{} : {}",
    //             block_info.height, block_info.time_ms, e
    //         )
    //     });
    //
    //     // println!("{:#?}", block_end_response);
    //
    //     let after_finalize_block_request = AfterFinalizeBlockRequest {
    //         updated_data_contract_ids: Vec::new(),
    //     };
    //
    //     self.after_finalize_block(after_finalize_block_request)
    //         .unwrap_or_else(|_| {
    //             panic!(
    //                 "should begin process block #{} at time #{}",
    //                 block_info.height, block_info.time_ms
    //             )
    //         });
    //
    //     // TODO: Move to `after_finalize_block` so it will be called by JS Drive too
    //     self.state.write().unwrap().last_block_info = Some(block_info.clone());
    //
    //     Ok(())
    // }

    pub(crate) fn process_raw_state_transitions(
        &self,
        raw_state_transitions: &Vec<Vec<u8>>,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<(FeeResult, Vec<ExecTxResult>), Error> {
        let state_transitions = StateTransition::deserialize_many(raw_state_transitions)?;

        state_transitions
            .into_iter()
            .map(|state_transition| {
                let state_transition_execution_event =
                    state_transition.validate_state_transition(self)?;
                // we map the result to the actual execution
                let execution_result: ExecutionResult = state_transition_execution_event
                    .map_result(|execution_event| {
                        self.execute_event(execution_event, block_info, transaction)
                    })?
                    .into();
                let fee_result = if let SuccessfulPaidExecution(_, fee_result) = execution_result {
                    Some(fee_result)
                } else {
                    None
                };

                (fee_result, execution_result.into())
            })
            .unzip()
            .collect::<Result<(FeeResult, Vec<ExecTxResult>), Error>>()
    }

    pub fn run_block_proposal(
        &self,
        block_proposal: BlockProposal,
        transaction: &Transaction,
    ) -> Result<BlockExecutionOutcome, Error> {
        let BlockProposal {
            height,
            round,
            core_chain_locked_height,
            proposed_app_version,
            proposer_pro_tx_hash,
            validator_set_quorum_hash,
            block_time_ms,
            raw_state_transitions,
        } = block_proposal;
        // We start by getting the epoch we are in
        let genesis_time_ms = self.get_genesis_time(height, block_time_ms, &transaction)?;
        // Init block execution context
        let block_state_info = BlockStateInfo::from_block_proposal(&block_proposal);

        let epoch_info =
            EpochInfo::from_genesis_time_and_block_info(genesis_time_ms, &block_state_info)?;

        //
        self.drive
            .update_validator_proposed_app_version(
                proposer_pro_tx_hash,
                proposed_app_version as u32,
                Some(&transaction),
            )
            .map_err(|e| format!("cannot update proposed app version: {}", e))?;

        let block_info = block_state_info.to_block_info(epoch_info.current_epoch_index);
        // FIXME: we need to calculate total hpmns based on masternode list (or remove hpmn_count if not needed)
        let total_hpmns = self.config.quorum_size as u32;
        let mut block_execution_context = BlockExecutionContext {
            current_transaction: transaction,
            block_info: block_state_info,
            epoch_info: epoch_info.clone(),
            hpmn_count: total_hpmns,
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
        let last_synced_core_height = block_execution_context.block_info.core_chain_locked_height;

        self.update_broadcasted_withdrawal_transaction_statuses(
            last_synced_core_height,
            &block_execution_context,
            &transaction,
        )?;

        let unsigned_withdrawal_transaction_bytes = self
            .fetch_and_prepare_unsigned_withdrawal_transactions(
                validator_set_quorum_hash,
                &block_execution_context,
                &transaction,
            )?;

        let (block_fees, tx_results) =
            self.process_raw_state_transitions(&raw_state_transitions, &block_info, transaction)?;

        self.pool_withdrawals_into_transactions_queue(&block_execution_context)?;

        // while we have the state transitions executed, we now need to process the block fees

        // Process fees
        let process_block_fees_outcome = self.process_block_fees(
            &block_state_info,
            &epoch_info,
            block_fees.into(),
            transaction,
        )?;

        let root_hash = self.drive.grove.root_hash(Some(&transaction)).unwrap()?;

        block_execution_context.block_info.commit_hash = Some(root_hash);

        Ok(BlockExecutionOutcome {
            block_execution_context,
            tx_results,
        })
    }
}
