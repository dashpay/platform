// TODO: replace `handlers.rs` with this file

use crate::{
    block::{BlockExecutionContext, BlockStateInfo},
    error::{execution::ExecutionError, Error},
    execution::fee_pools::epoch::EpochInfo,
    platform::Platform,
};

use dpp::util::vec::vec_to_array;
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

impl Proposal for Platform {
    fn prepare_proposal(
        &self,
        request: &proto::RequestPrepareProposal,
        transaction: TransactionArg,
    ) -> Result<proto::ResponsePrepareProposal, ResponseException> {
        let genesis_time_ms = if request.height == self.config.abci.genesis_height {
            let block_time_ms = request
                .time
                .as_ref()
                .ok_or("missing proposal time")?
                .to_milis();
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

        // Update versions
        let proposed_app_version = request.proposed_app_version;

        self.drive
            .update_validator_proposed_app_version(
                vec_to_array(&request.proposer_pro_tx_hash)
                    .map_err(|e| format!("invalid proposer protxhash: {}", e))?,
                proposed_app_version as u32,
                transaction,
            )
            .map_err(|e| format!("cannot update proposed app version: {}", e))?;

        // Init block execution context
        let block_info = BlockStateInfo::from_prepare_proposal_request(&request);

        let epoch_info = EpochInfo::from_genesis_time_and_block_info(genesis_time_ms, &block_info)?;

        // FIXME: we need to calculate total hpms based on masternode list (or remove hpmn_count if not needed)
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
            .replace(Some(block_execution_context));

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

        let mut tx_results = ::prost::alloc::vec::Vec::<ExecTxResult>::new();
        for tx in request.txs.clone() {
            tx_results.push(mock_exec_tx(tx)) // TODO: execute transactions in a proper way
        }
        // TODO: implement all fields, including tx processing; for now, just leaving bare minimum
        let response = proto::ResponsePrepareProposal {
            app_hash: vec![0; 32], // TODO: implement
            tx_results,
            ..Default::default()
        };

        Ok(response)
    }
}

/// Return tx result that just copies tx to data field
fn mock_exec_tx(tx: Vec<u8>) -> ExecTxResult {
    ExecTxResult {
        code: 0,
        data: tx.clone(),
        ..Default::default()
    }
}
