use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::data_contracts::withdrawals_contract::WithdrawalStatus;
use dpp::version::PlatformVersion;
use drive::drive::identity::withdrawals::paths::get_withdrawal_transactions_queue_path_vec;
use drive::grovedb::{PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use drive::query::QueryResultType;
use std::ops::RangeFull;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Looks at one element of the untied withdrawal transactions queue and at one EXPIRED
    /// withdrawal document; either existing means the next block has withdrawal work.
    pub(super) fn has_pending_withdrawal_work_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let mut query = Query::new();
        query.insert_item(QueryItem::RangeFull(RangeFull));
        let path_query = PathQuery {
            path: get_withdrawal_transactions_queue_path_vec(),
            query: SizedQuery {
                query,
                limit: Some(1),
                offset: None,
            },
        };

        let (queued_transactions, _) = self.drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        if !queued_transactions.is_empty() {
            return Ok(true);
        }

        let expired_documents = self.drive.fetch_oldest_withdrawal_documents_by_status(
            WithdrawalStatus::EXPIRED.into(),
            1,
            transaction,
            platform_version,
        )?;

        Ok(!expired_documents.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContract;
    use dpp::data_contracts::withdrawals_contract::WithdrawalStatus;
    use dpp::data_contracts::SystemDataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::version::PlatformVersion;
    use dpp::withdrawal::Pooling;
    use drive::grovedb::Transaction;
    use drive::util::test_helpers::setup::{setup_document, setup_system_data_contract};

    fn setup_platform(
        platform_version: &PlatformVersion,
    ) -> (TempPlatform<MockCoreRPCLike>, DataContract) {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("expected to load the withdrawals contract");

        (platform, data_contract)
    }

    fn insert_withdrawal_document(
        platform: &TempPlatform<MockCoreRPCLike>,
        data_contract: &DataContract,
        status: WithdrawalStatus,
        transaction_index: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) {
        let document = get_withdrawal_document_fixture(
            data_contract,
            Identifier::new([1u8; 32]),
            platform_value!({
                "amount": 1_000_000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": status as u8,
                "transactionIndex": transaction_index,
                "transactionSignHeight": 1u64,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected a withdrawal document");

        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected the withdrawal document type");

        setup_document(
            &platform.drive,
            &document,
            data_contract,
            document_type,
            Some(transaction),
        );
    }

    #[test]
    fn nothing_is_pending_on_a_fresh_state() {
        let platform_version = PlatformVersion::latest();
        let (platform, data_contract) = setup_platform(platform_version);
        let transaction = platform.drive.grove.start_transaction();
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

        assert!(!platform
            .has_pending_withdrawal_work(Some(&transaction), platform_version)
            .expect("expected to check for pending withdrawal work"));
    }

    #[test]
    fn queued_transactions_are_pending_until_they_are_dequeued() {
        let platform_version = PlatformVersion::latest();
        let (platform, data_contract) = setup_platform(platform_version);
        let transaction = platform.drive.grove.start_transaction();
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));
        let block_info = BlockInfo::default();

        // Pooling puts the untied transaction bytes into the queue.
        let mut drive_operations = vec![];
        platform
            .drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                vec![(1, vec![7u8; 32])],
                1_000_000,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to enqueue a withdrawal transaction");
        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply the enqueue operations");

        assert!(platform
            .has_pending_withdrawal_work(Some(&transaction), platform_version)
            .expect("expected to check for pending withdrawal work"));

        // The next block dequeues it for signing, which moves the bytes to the broadcasted
        // subtree; those wait on Core, not on another Platform block.
        let mut drive_operations = vec![];
        let dequeued = platform
            .drive
            .dequeue_untied_withdrawal_transactions(
                4,
                Some(&transaction),
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to dequeue withdrawal transactions");
        assert_eq!(dequeued.len(), 1);
        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply the dequeue operations");

        assert!(!platform
            .has_pending_withdrawal_work(Some(&transaction), platform_version)
            .expect("expected to check for pending withdrawal work"));
    }

    #[test]
    fn expired_documents_are_pending_but_broadcasted_and_complete_ones_are_not() {
        let platform_version = PlatformVersion::latest();
        let (platform, data_contract) = setup_platform(platform_version);
        let transaction = platform.drive.grove.start_transaction();
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

        insert_withdrawal_document(
            &platform,
            &data_contract,
            WithdrawalStatus::BROADCASTED,
            1,
            &transaction,
            platform_version,
        );
        insert_withdrawal_document(
            &platform,
            &data_contract,
            WithdrawalStatus::COMPLETE,
            2,
            &transaction,
            platform_version,
        );

        assert!(!platform
            .has_pending_withdrawal_work(Some(&transaction), platform_version)
            .expect("expected to check for pending withdrawal work"));

        // An expired withdrawal is moved back to the queue and re-signed by the next block.
        insert_withdrawal_document(
            &platform,
            &data_contract,
            WithdrawalStatus::EXPIRED,
            3,
            &transaction,
            platform_version,
        );

        assert!(platform
            .has_pending_withdrawal_work(Some(&transaction), platform_version)
            .expect("expected to check for pending withdrawal work"));
    }
}
