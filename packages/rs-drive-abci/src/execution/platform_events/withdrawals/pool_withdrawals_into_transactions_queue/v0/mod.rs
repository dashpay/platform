use dpp::block::block_info::BlockInfo;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

use dpp::system_data_contracts::withdrawals_contract;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use drive::config::DEFAULT_QUERY_LIMIT;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
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
    pub(super) fn pool_withdrawals_into_transactions_queue_v0(
        &self,
        block_info: &BlockInfo,
        last_committed_platform_state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Currently Core only supports using the first 2 quorums (out of 24 for mainnet).
        // For us, we just use the latest quorum to be extra safe.
        let Some(position_of_current_quorum) =
            last_committed_platform_state.current_validator_set_position_in_list_by_most_recent()
        else {
            tracing::warn!("Current quorum not in current validator set, not making withdrawals");
            return Ok(());
        };
        if position_of_current_quorum != 0 {
            tracing::debug!(
                "Current quorum is not most recent, it is in position {}, not making withdrawals",
                position_of_current_quorum
            );
            return Ok(());
        }
        let documents = self.drive.fetch_oldest_withdrawal_documents_by_status(
            withdrawals_contract::WithdrawalStatus::QUEUED.into(),
            DEFAULT_QUERY_LIMIT,
            transaction,
            platform_version,
        )?;

        if documents.is_empty() {
            return Ok(());
        }

        // Only take documents up to the withdrawal amount
        let current_withdrawal_limit = self
            .drive
            .calculate_current_withdrawal_limit(transaction, platform_version)?;

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
            if total_withdrawal_amount + amount > current_withdrawal_limit {
                // If adding this withdrawal would exceed the limit, stop processing further.
                break;
            }

            // Add this document to the list of documents to be processed.
            documents_to_process.push(document);
            total_withdrawal_amount += amount;
        }

        if documents_to_process.is_empty() {
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
