use dashcore_rpc::dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo,
    hashes::Hash, QuorumHash,
};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::build_asset_unlock_tx;
use dpp::dashcore::Transaction;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Setters};
use dpp::version::PlatformVersion;

use drive::dpp::system_data_contracts::withdrawals_contract;
use drive::dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;

use drive::drive::identity::withdrawals::WithdrawalTransactionIndex;
use drive::query::TransactionArg;
use drive::util::batch::DriveOperation;

use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::CoreHeight;
use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};
use dpp::errors::ProtocolError;

use drive::config::DEFAULT_QUERY_LIMIT;

const WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT: u16 = 16;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Prepares a list of an unsigned withdrawal transaction bytes
    pub(super) fn dequeue_and_build_unsigned_withdrawal_transactions_v0(
        &self,
        validator_set_quorum_hash: [u8; 32],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<UnsignedWithdrawalTxs, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];

        // Get 16 latest withdrawal transactions from the queue
        let untied_withdrawal_transactions = self.drive.dequeue_untied_withdrawal_transactions(
            WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        if untied_withdrawal_transactions.is_empty() {
            return Ok(UnsignedWithdrawalTxs::default());
        }

        let transaction_indices = untied_withdrawal_transactions
            .iter()
            .map(|(transaction_id, _)| *transaction_id)
            .collect::<Vec<_>>();

        let documents = self.fetch_and_modify_withdrawal_documents_to_broadcasted_by_indices(
            &transaction_indices,
            block_info,
            transaction,
            platform_version,
        )?;

        tracing::debug!(
            "Deque {} unsigned withdrawal transactions for signing with indices from {} to {}",
            documents.len(),
            transaction_indices.first().expect("must be present"),
            transaction_indices.last().expect("must be present")
        );

        let withdrawals_contract = self.drive.cache.system_data_contracts.load_withdrawals();

        self.drive.add_update_multiple_documents_operations(
            &documents,
            &withdrawals_contract,
            withdrawals_contract
                .document_type_for_name(withdrawal::NAME)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "could not get document type",
                    ))
                })?,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // Appending request_height and quorum_hash to withdrawal transaction
        let unsigned_withdrawal_transactions = untied_withdrawal_transactions
            .into_iter()
            .map(|(_, untied_transaction_bytes)| {
                build_unsigned_transaction(
                    untied_transaction_bytes,
                    validator_set_quorum_hash,
                    block_info.core_height,
                )
            })
            .collect::<Result<_, _>>()?;

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            block_info,
            transaction,
            platform_version,
            None,
        )?;

        Ok(UnsignedWithdrawalTxs::from_vec(
            unsigned_withdrawal_transactions,
        ))
    }

    fn fetch_and_modify_withdrawal_documents_to_broadcasted_by_indices(
        &self,
        transaction_indices: &[WithdrawalTransactionIndex],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        let documents = self
            .drive
            .find_withdrawal_documents_by_status_and_transaction_indices(
                withdrawals_contract::WithdrawalStatus::POOLED,
                transaction_indices,
                DEFAULT_QUERY_LIMIT,
                transaction,
                platform_version,
            )?;

        documents
            .into_iter()
            .map(|mut document| {
                document.set_i64(
                    withdrawal::properties::STATUS,
                    withdrawals_contract::WithdrawalStatus::BROADCASTED as i64,
                );

                document.set_u64(
                    withdrawal::properties::TRANSACTION_SIGN_HEIGHT,
                    block_info.core_height as u64,
                );

                document.set_updated_at(Some(block_info.time_ms));

                document.increment_revision().map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Could not increment document revision",
                    ))
                })?;

                Ok(document)
            })
            .collect()
    }
}

fn build_unsigned_transaction(
    untied_transaction_bytes: Vec<u8>,
    mut validator_set_quorum_hash: [u8; 32],
    core_chain_locked_height: CoreHeight,
) -> Result<Transaction, Error> {
    // Core expects it reversed
    validator_set_quorum_hash.reverse();

    let request_info = AssetUnlockRequestInfo {
        request_height: core_chain_locked_height,
        quorum_hash: QuorumHash::from_byte_array(validator_set_quorum_hash),
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

    build_asset_unlock_tx(&unsigned_transaction_bytes)
        .map_err(|error| Error::Protocol(ProtocolError::DashCoreError(error)))
}
