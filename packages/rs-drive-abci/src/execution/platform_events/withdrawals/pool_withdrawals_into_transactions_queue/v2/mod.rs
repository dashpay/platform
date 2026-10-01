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
    GAUGE_CREDIT_WITHDRAWAL_LIMIT_AVAILABLE, GAUGE_CREDIT_WITHDRAWAL_LIMIT_CORE_AVAILABLE,
    GAUGE_CREDIT_WITHDRAWAL_LIMIT_TOTAL,
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
    /// Pool withdrawal documents into transactions.
    ///
    /// Version 2 differs from version 1 only in the amount it pools up to: the smaller of the
    /// daily withdrawal limit and the Core-anchored limit, a stricter copy of Core's own asset
    /// unlock rule read from Core's credit pool balances. Platform's own accounting can grant
    /// more than Core will mine (an asset lock published to Platform long after Core mined it,
    /// the epoch Core rewards minted in one block); an unlock over Core's limit waits unmined,
    /// expires and is re-signed, and while Core's mempool holds more than the limit, Core
    /// InstantSend-locks no withdrawal at all. What the Core side holds back stays queued on
    /// Platform instead. It first reads the Core blocks the chain locked height passed
    /// (`scan_core_blocks_for_withdrawals`).
    pub(super) fn pool_withdrawals_into_transactions_queue_v2(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Bring the Core blocks up to date first, every block whether or not anything is
        // queued: their credit pool balances feed the Core-anchored limit below, the asset
        // locks they mined are dated for the daily limit, and a long jump of the chain locked
        // height is read over several blocks.
        self.scan_core_blocks_for_withdrawals(block_info, transaction, platform_version)?;

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
            return Ok(());
        }

        // Only take documents up to the withdrawal amount
        let withdrawals_info = self.drive.calculate_current_withdrawal_limit(
            block_info,
            transaction,
            platform_version,
        )?;

        tracing::trace!(
            ?withdrawals_info,
            documents_count = documents.len(),
            "Calculated withdrawal limit info"
        );

        let core_anchored_withdrawal_limit = self.calculate_core_anchored_withdrawal_limit(
            block_info,
            transaction,
            platform_version,
        )?;

        tracing::trace!(
            core_anchored_withdrawal_limit,
            "Calculated Core-anchored withdrawal limit"
        );

        let current_withdrawal_limit = withdrawals_info
            .available()
            .min(core_anchored_withdrawal_limit);

        // Store prometheus metrics
        gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_AVAILABLE).set(withdrawals_info.available() as f64);
        gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_TOTAL).set(withdrawals_info.daily_maximum as f64);
        gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_CORE_AVAILABLE)
            .set(core_anchored_withdrawal_limit as f64);

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

        let withdrawals_contract = self
            .drive
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)?;

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
    use crate::rpc::core::{CoreCreditPoolBlock, MockCoreRPCLike};
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::epoch::Epoch;
    use dpp::data_contracts::SystemDataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::withdrawal::Pooling;
    use drive::config::DEFAULT_QUERY_LIMIT;
    use drive::util::test_helpers::setup::{setup_document, setup_system_data_contract};

    /// Pools two queued withdrawals of 1,000 credits each against a Core whose credit pool
    /// holds `pool_duffs` at every height, and returns how many were pooled.
    fn pooled_with_a_core_pool_of(pool_duffs: u64) -> usize {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc.expect_get_credit_pool_block().returning(move |_| {
            Ok(CoreCreditPoolBlock {
                credit_pool_balance: pool_duffs,
                asset_lock_txids: vec![],
            })
        });
        platform.core_rpc = core_rpc;

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 10_000,
            epoch: Epoch::default(),
        };

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));
        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get document type");

        for transaction_index in [1u64, 2] {
            let document = get_withdrawal_document_fixture(
                &data_contract,
                Identifier::new([1u8; 32]),
                platform_value!({
                    "amount": 1000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                    "transactionIndex": transaction_index,
                }),
                None,
                platform_version.protocol_version,
            )
            .expect("expected withdrawal document");
            setup_document(
                &platform.drive,
                &document,
                &data_contract,
                document_type,
                Some(&transaction),
            );
        }

        platform
            .pool_withdrawals_into_transactions_queue_v2(
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("to pool withdrawal documents into transactions");

        platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::POOLED.into(),
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch withdrawal documents")
            .len()
    }

    #[test]
    fn should_pool_what_fits_both_the_daily_limit_and_cores_credit_pool() {
        // Plenty in Core's pool: both withdrawals pool.
        assert_eq!(pooled_with_a_core_pool_of(1_000_000_000_000), 2);
    }

    #[test]
    fn should_leave_queued_what_cores_credit_pool_could_not_give_up() {
        // A pool of 1 duff, 1,000 credits: it fits one 1,000 credit withdrawal, not two,
        // although the daily limit (2,000 Dash before any history) would allow both.
        assert_eq!(pooled_with_a_core_pool_of(1), 1);
        assert_eq!(pooled_with_a_core_pool_of(0), 0);
    }
}
