use dpp::block::block_info::BlockInfo;
use metrics::gauge;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

use dpp::system_data_contracts::withdrawals_contract;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;

use crate::metrics::{
    GAUGE_CREDIT_WITHDRAWAL_LIMIT_AVAILABLE, GAUGE_CREDIT_WITHDRAWAL_LIMIT_TOTAL,
};
use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Pool withdrawal documents into transactions
    /// Version 1 changes on Version 0, by not having the Core 2 Quorum limit.
    /// We should switch to Version 1 once Core has fixed the issue
    pub(super) fn pool_withdrawals_into_transactions_queue_v1(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let documents = self.drive.fetch_oldest_withdrawal_documents_by_status(
            withdrawals_contract::WithdrawalStatus::QUEUED.into(),
            platform_version
                .system_limits
                .withdrawal_transactions_per_block_limit,
            transaction,
            platform_version,
        )?;

        if documents.is_empty() {
            tracing::debug!(
                height = block_info.height,
                withdrawal_limit = platform_version
                    .system_limits
                    .withdrawal_transactions_per_block_limit,
                "No queued withdrawal documents found to pool into transactions"
            );
            let all_documents = self
                .drive
                .fetch_oldest_withdrawal_documents(transaction, platform_version)?;
            if all_documents.is_empty() {
                tracing::debug!(
                    height = block_info.height,
                    "No withdrawal documents found at all"
                );
            } else {
                // Count documents by status
                let queued_count = all_documents
                    .get(&(withdrawals_contract::WithdrawalStatus::QUEUED as u8))
                    .map(|v| v.len())
                    .unwrap_or(0);
                let pooled_count = all_documents
                    .get(&(withdrawals_contract::WithdrawalStatus::POOLED as u8))
                    .map(|v| v.len())
                    .unwrap_or(0);
                let broadcasted_count = all_documents
                    .get(&(withdrawals_contract::WithdrawalStatus::BROADCASTED as u8))
                    .map(|v| v.len())
                    .unwrap_or(0);
                let complete_count = all_documents
                    .get(&(withdrawals_contract::WithdrawalStatus::COMPLETE as u8))
                    .map(|v| v.len())
                    .unwrap_or(0);
                let expired_count = all_documents
                    .get(&(withdrawals_contract::WithdrawalStatus::EXPIRED as u8))
                    .map(|v| v.len())
                    .unwrap_or(0);
                let total_documents = queued_count
                    + pooled_count
                    + broadcasted_count
                    + complete_count
                    + expired_count;

                tracing::debug!(
                    height = block_info.height,
                    total_documents,
                    queued_count,
                    pooled_count,
                    broadcasted_count,
                    complete_count,
                    expired_count,
                    "Found withdrawal documents grouped by status"
                );
            }
            return Ok(());
        }

        // Only take documents up to the withdrawal amount
        let withdrawals_info = self
            .drive
            .calculate_current_withdrawal_limit(transaction, platform_version)?;

        tracing::trace!(
            ?withdrawals_info,
            documents_count = documents.len(),
            "Calculated withdrawal limit info"
        );

        let current_withdrawal_limit = withdrawals_info.available();

        // Store prometheus metrics
        gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_AVAILABLE).set(current_withdrawal_limit as f64);
        gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_TOTAL).set(withdrawals_info.daily_maximum as f64);

        // Only process documents up to the current withdrawal limit.
        let mut total_withdrawal_amount = 0u64;

        // Iterate over the documents and accumulate their withdrawal amounts.
        let mut documents_to_process = vec![];
        for document in documents {
            // Get the withdrawal amount from the document properties.
            let amount: u64 = document
                .properties()
                .get_integer(withdrawal::properties::AMOUNT)?;

            // Check if adding this amount would exceed the current withdrawal limit.
            let potential_total_withdrawal_amount =
                total_withdrawal_amount.checked_add(amount).ok_or_else(|| {
                    Error::Execution(ExecutionError::Overflow(
                        "overflow in total withdrawal amount",
                    ))
                })?;

            // If adding this withdrawal would exceed the limit, stop further processing.
            if potential_total_withdrawal_amount > current_withdrawal_limit {
                tracing::debug!(
                    "Pooling is limited due to daily withdrawals limit. {} credits left",
                    current_withdrawal_limit
                );
                break;
            }

            total_withdrawal_amount = potential_total_withdrawal_amount;

            // Add this document to the list of documents to be processed.
            documents_to_process.push(document);
        }

        if documents_to_process.is_empty() {
            tracing::debug!(
                block_info = %block_info,
                "No withdrawal documents to process"
            );
            return Ok(());
        }

        let start_transaction_index = self
            .drive
            .fetch_next_withdrawal_transaction_index(transaction, platform_version)?;

        let (withdrawal_transactions, total_amount) = self
            .build_untied_withdrawal_transactions_from_documents(
                &mut documents_to_process,
                start_transaction_index,
                block_info,
                platform_version,
            )?;

        let withdrawal_transactions_count = withdrawal_transactions.len();

        let mut drive_operations = vec![];

        self.drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                withdrawal_transactions,
                total_amount,
                &mut drive_operations,
                platform_version,
            )?;

        let end_transaction_index = start_transaction_index + withdrawal_transactions_count as u64;

        self.drive
            .add_update_next_withdrawal_transaction_index_operation(
                end_transaction_index,
                &mut drive_operations,
                platform_version,
            )?;

        tracing::debug!(
            "Pooled {} withdrawal documents into {} transactions with indices from {} to {}",
            documents_to_process.len(),
            withdrawal_transactions_count,
            start_transaction_index,
            end_transaction_index,
        );

        let withdrawals_contract = self.drive.cache.system_data_contracts.load_withdrawals();

        self.drive.add_update_multiple_documents_operations(
            &documents_to_process,
            &withdrawals_contract,
            withdrawals_contract
                .document_type_for_name(withdrawal::NAME)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })?,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            block_info,
            transaction,
            platform_version,
            None,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::block::epoch::Epoch;
    use itertools::Itertools;

    use dpp::data_contracts::SystemDataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::withdrawal::Pooling;
    use drive::util::test_helpers::setup::{setup_document, setup_system_data_contract};

    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;
    use drive::config::DEFAULT_QUERY_LIMIT;

    #[test]
    fn test_pooling() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 96,
            epoch: Epoch::default(),
        };

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");

        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

        let owner_id = Identifier::new([1u8; 32]);

        let document_1 = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                "transactionIndex": 1u64,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get document type");

        setup_document(
            &platform.drive,
            &document_1,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        let document_2 = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                "transactionIndex": 2u64,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

        setup_document(
            &platform.drive,
            &document_2,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        let platform_state = platform.state.load();

        platform
            .pool_withdrawals_into_transactions_queue_v0(
                &block_info,
                &platform_state,
                Some(&transaction),
                platform_version,
            )
            .expect("to pool withdrawal documents into transactions");

        let updated_documents = platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::POOLED.into(),
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch withdrawal documents");

        for (i, document) in updated_documents
            .into_iter()
            // Sort by index because updated_at is the same for all documents within batch
            .sorted_by(|a, b| {
                let a_index = a
                    .properties()
                    .get_u64(withdrawal::properties::TRANSACTION_INDEX)
                    .expect("to get transactionIndex");
                let b_index = b
                    .properties()
                    .get_u64(withdrawal::properties::TRANSACTION_INDEX)
                    .expect("to get transactionIndex");
                a_index.cmp(&b_index)
            })
            .enumerate()
        {
            assert_eq!(document.revision(), Some(2));

            let tx_index = document
                .properties()
                .get_u64(withdrawal::properties::TRANSACTION_INDEX)
                .expect("to get transactionIndex");

            assert_eq!(tx_index, i as u64);
        }
    }
}
