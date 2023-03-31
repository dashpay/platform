// TODO: replace `handlers.rs` with this file

use crate::{
    block::{BlockExecutionContext, BlockStateInfo},
    error::{execution::ExecutionError, Error},
    execution::fee_pools::epoch::EpochInfo,
    platform::Platform,
    rpc::core::CoreRPCLike,
};

use crate::validation::state_transition::StateTransitionValidation;
use crate::validation::state_transition_action::StateTransitionActionValidation;
use dpp::prelude::{Identifier, ValidationResult};
use dpp::state_transition::StateTransition;
use dpp::util::vec::vec_to_array;
use drive::drive::batch::transitions::DriveHighLevelOperationConverter;
use drive::drive::batch::DriveOperation;
use drive::fee_pools::epochs::Epoch;
use drive::query::TransactionArg;
use tenderdash_abci::proto::{
    abci::{self as proto, ExecTxResult, ResponseException},
    serializers::timestamp::ToMilis,
};

use super::AbciError;

pub trait Proposal {
    fn prepare_proposal(
        &self,
        request: &proto::RequestPrepareProposal,
        transaction: TransactionArg,
    ) -> Result<proto::ResponsePrepareProposal, ResponseException>;
}

impl<'a, C> Proposal for Platform<'a, C>
where
    C: CoreRPCLike,
{
    fn prepare_proposal(
        &self,
        request: proto::RequestPrepareProposal,
        transaction: TransactionArg,
    ) -> Result<proto::ResponsePrepareProposal, ResponseException> {
        let proto::RequestPrepareProposal {
            max_tx_bytes,
            txs,
            local_last_commit,
            misbehavior,
            height,
            time,
            next_validators_hash,
            round,
            core_chain_locked_height,
            proposer_pro_tx_hash,
            proposed_app_version,
            version,
            quorum_hash,
        } = request;

        let genesis_time_ms = if height == self.config.abci.genesis_height {
            let block_time_ms = time.as_ref().ok_or("missing proposal time")?.to_milis();
            self.drive.set_genesis_time(block_time_ms);
            block_time_ms
        } else {
            //todo: lazy load genesis time
            self.drive
                .get_genesis_time(transaction)
                .map_err(Error::Drive)?
                .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                    "the genesis time must be set",
                )))?
        };

        let validator_pro_tx_hash: [u8; 32] = proposer_pro_tx_hash
            .try_into()
            .map_err(|e| format!("invalid proposer protxhash: {}", hex::encode(e)))?;

        self.drive
            .update_validator_proposed_app_version(
                validator_pro_tx_hash,
                proposed_app_version as u32,
                transaction,
            )
            .map_err(|e| format!("cannot update proposed app version: {}", e))?;

        // Init block execution context
        let block_info = BlockStateInfo::from_prepare_proposal_request(&request);

        let epoch_info = EpochInfo::from_genesis_time_and_block_info(genesis_time_ms, &block_info)?;

        // FIXME: we need to calculate total hpmns based on masternode list (or remove hpmn_count if not needed)
        let total_hpmns = self.config.quorum_size as u32;
        let block_execution_context = BlockExecutionContext {
            block_info,
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

        self.block_execution_context
            .write()
            .unwrap()
            .replace(block_execution_context);

        self.update_broadcasted_withdrawal_transaction_statuses(
            last_synced_core_height,
            transaction,
        )?;

        // TODO: restore withdrawal transactions logic
        // let unsigned_withdrawal_transaction_bytes = self
        //     .fetch_and_prepare_unsigned_withdrawal_transactions(
        //         vec_to_array(&request.quorum_hash).expect("invalid quorum hash"),
        //         transaction,
        //     )?;

        let state_transitions = StateTransition::deserialize_many(&txs)?;

        let validation_outcomes = state_transitions
            .into_iter()
            .map(|state_transition| {
                let state_transition_action_result = state_transition.validate_all(self)?;
                state_transition_action_result
                    .and_then_simple_validation(|action| action.validate_fee(&self.drive))?
                    .map_result(|state_transition| {
                        state_transition.into_high_level_drive_operations(&Epoch::new(
                            epoch_info.current_epoch_index,
                        ))
                    })?
                    .map_result(|state_transition| {
                        state_transition.into_high_level_drive_operations(&Epoch::new(
                            epoch_info.current_epoch_index,
                        ))
                    })
            })
            .collect::<Result<Vec<ValidationResult<Vec<DriveOperation>>>, Error>>()?;

        // TODO: implement all fields, including tx processing; for now, just leaving bare minimum
        let response = proto::ResponsePrepareProposal {
            app_hash: vec![0; 32], // TODO: implement
            tx_results,
            ..Default::default()
        };

        Ok(response)
    }
}
