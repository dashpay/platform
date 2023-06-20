use std::ops::Deref;

use dashcore_rpc::dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo,
    hashes::Hash, QuorumHash,
};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::document::Document;

use drive::dpp::contracts::withdrawals_contract;

use drive::dpp::util::hash;

use drive::drive::batch::DriveOperation;
use drive::grovedb::Transaction;

use crate::execution::types::block_execution_context;
use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};

const WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT: u16 = 16;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Prepares a list of an unsigned withdrawal transaction bytes
    pub fn fetch_and_prepare_unsigned_withdrawal_transactions_v0(
        &self,
        validator_set_quorum_hash: [u8; 32],
        block_execution_context: &block_execution_context::v0::BlockExecutionContext,
        transaction: &Transaction,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let block_info = BlockInfo {
            time_ms: block_execution_context.block_state_info.block_time_ms,
            height: block_execution_context.block_state_info.height,
            core_height: block_execution_context
                .block_state_info
                .core_chain_locked_height,
            epoch: Epoch::new(block_execution_context.epoch_info.current_epoch_index)?,
        };

        let data_contract_id = withdrawals_contract::CONTRACT_ID.deref();

        let (_, Some(contract_fetch_info)) = self.drive.get_contract_with_fetch_info_and_fee(
            data_contract_id.to_buffer(),
            None,
            true,
            Some(transaction),
        )? else {
            return Err(Error::Execution(
                ExecutionError::CorruptedCodeExecution("can't fetch withdrawal data contract"),
            ));
        };

        let mut drive_operations: Vec<DriveOperation> = vec![];

        // Get 16 latest withdrawal transactions from the queue
        let untied_withdrawal_transactions = self.drive.dequeue_withdrawal_transactions(
            WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT,
            Some(transaction),
            &mut drive_operations,
        )?;

        if untied_withdrawal_transactions.is_empty() {
            return Ok(Vec::new());
        }

        // Appending request_height and quorum_hash to withdrawal transaction
        // and pass it to JS Drive for singing and broadcasting
        let (unsigned_withdrawal_transactions, documents_to_update): (Vec<_>, Vec<_>) =
            untied_withdrawal_transactions
                .into_iter()
                .map(|(_, untied_transaction_bytes)| {
                    let request_info = AssetUnlockRequestInfo {
                        request_height: block_execution_context
                            .block_state_info
                            .core_chain_locked_height,
                        quorum_hash: QuorumHash::hash(&validator_set_quorum_hash),
                    };

                    let mut unsigned_transaction_bytes = vec![];

                    request_info
                        .consensus_append_to_base_encode(
                            untied_transaction_bytes.clone(),
                            &mut unsigned_transaction_bytes,
                        )
                        .map_err(|_| {
                            Error::Execution(ExecutionError::CorruptedCodeExecution(
                                "could not add additional request info to asset unlock transaction",
                            ))
                        })?;

                    let original_transaction_id = hash::hash_to_vec(untied_transaction_bytes);
                    let update_transaction_id =
                        hash::hash_to_vec(unsigned_transaction_bytes.clone());

                    let mut document = self.drive.find_withdrawal_document_by_transaction_id(
                        &original_transaction_id,
                        Some(transaction),
                    )?;

                    document.set_bytes(
                        withdrawals_contract::property_names::TRANSACTION_ID,
                        update_transaction_id,
                    );

                    document.set_i64(
                        withdrawals_contract::property_names::UPDATED_AT,
                        block_info.time_ms.try_into().map_err(|_| {
                            Error::Execution(ExecutionError::CorruptedCodeExecution(
                                "Can't convert u64 block time to i64 updated_at",
                            ))
                        })?,
                    );

                    document.increment_revision().map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Could not increment document revision",
                        ))
                    })?;

                    Ok((unsigned_transaction_bytes, document))
                })
                .collect::<Result<Vec<(Vec<u8>, Document)>, Error>>()?
                .into_iter()
                .unzip();

        self.drive.add_update_multiple_documents_operations(
            &documents_to_update,
            &contract_fetch_info.contract,
            contract_fetch_info
                .contract
                .document_type_for_name(withdrawals_contract::document_types::WITHDRAWAL)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "could not get document type",
                    ))
                })?,
            &mut drive_operations,
        );

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            &block_info,
            Some(transaction),
        )?;

        Ok(unsigned_withdrawal_transactions)
    }
}
