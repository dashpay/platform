use dpp::block::block_info::BlockInfo;
use dpp::identity::convert_duffs_to_credits;
use dpp::version::PlatformVersion;
use drive::drive::identity::withdrawals::calculate_current_withdrawal_limit::CoreCreditPoolSnapshot;
use drive::grovedb::TransactionArg;

use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Pools queued withdrawals against the limit derived from Core's credit pool at the
    /// block's chain locked height. Version 2 differs from version 1 only in where the limit
    /// comes from: `getcreditpoolinfo` at `block_info.core_height` gives the pool balance, the
    /// balance one window earlier and Core's own limit for the next block, from which
    /// `calculate_current_withdrawal_limit` v1 derives Platform's share; the pooled withdrawals
    /// still in flight count against it.
    ///
    /// The chain locked block is the same for every validator, so the values are
    /// deterministic; a Core that cannot answer is an execution error, as for the asset
    /// unlock statuses.
    pub(super) fn pool_withdrawals_into_transactions_queue_v2(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Ask Core for the pool only when there is something to pool: the shared body fetches
        // the queued documents again, which is cheaper than a Core round trip every block
        let has_queued_documents = !self
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                dpp::system_data_contracts::withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                1,
                transaction,
                platform_version,
            )?
            .is_empty();
        if !has_queued_documents {
            tracing::debug!(
                height = block_info.height,
                "No queued withdrawal documents found to pool into transactions"
            );
            return Ok(());
        }

        let core_credit_pool = self.fetch_core_credit_pool_snapshot(block_info.core_height)?;

        self.pool_withdrawals_into_transactions_queue_with_core_pool(
            block_info,
            Some(&core_credit_pool),
            transaction,
            platform_version,
        )
    }

    /// Reads Core's credit pool at a chain locked height, in credits
    fn fetch_core_credit_pool_snapshot(
        &self,
        core_chain_locked_height: u32,
    ) -> Result<CoreCreditPoolSnapshot, Error> {
        let info = self
            .core_rpc
            .get_credit_pool_info(core_chain_locked_height)?;
        if info.height != core_chain_locked_height {
            return Err(Error::Execution(ExecutionError::CorruptedDriveResponse(
                format!(
                    "Core reported the credit pool at height {} instead of {}",
                    info.height, core_chain_locked_height
                ),
            )));
        }
        let credits = |amount: dpp::dashcore::Amount| convert_duffs_to_credits(amount.to_sat());
        Ok(CoreCreditPoolSnapshot {
            balance: credits(info.balance)?,
            window_start_balance: credits(info.window_start_balance)?,
            current_limit: credits(info.current_limit)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::CreditPoolInfo;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::epoch::Epoch;
    use dpp::dashcore::Amount;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::DocumentV0Getters;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
    use dpp::system_data_contracts::{load_system_data_contract, withdrawals_contract};
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::withdrawal::Pooling;
    use dpp::{dash_to_credits, dash_to_duffs};
    use drive::config::DEFAULT_QUERY_LIMIT;
    use drive::drive::identity::withdrawals::paths::{
        get_withdrawal_root_path, WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
    };
    use drive::util::grove_operations::DirectQueryType;
    use drive::util::test_helpers::setup::{setup_document, setup_system_data_contract};

    fn credit_pool_info(
        height: u32,
        balance: u64,
        window_start_balance: u64,
        current_limit: u64,
    ) -> CreditPoolInfo {
        CreditPoolInfo {
            height,
            balance: Amount::from_sat(balance),
            current_limit: Amount::from_sat(current_limit),
            unlocked_in_window: Amount::from_sat(0),
            window_blocks: 576,
            window_start_height: height as i64 - 576,
            window_start_balance: Amount::from_sat(window_start_balance),
        }
    }

    #[test]
    fn should_pool_what_fits_the_limit_derived_from_cores_credit_pool() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        // 20,000 Dash in the pool one window ago and now: Platform's share is 15% = 3,000
        // Dash, Core's 20% = 4,000 Dash
        platform
            .core_rpc
            .expect_get_credit_pool_info()
            .returning(|height| {
                Ok(credit_pool_info(
                    height,
                    dash_to_duffs!(20000),
                    dash_to_duffs!(20000),
                    dash_to_duffs!(4000),
                ))
            });
        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 960,
            epoch: Epoch::default(),
        };
        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));
        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get document type");
        let owner_id = Identifier::new([1u8; 32]);
        // Three 1,200 Dash withdrawals: two fit the 3,000 Dash share, the third waits
        for i in 1..=3u64 {
            let document = get_withdrawal_document_fixture(
                &data_contract,
                owner_id,
                platform_value!({
                    "amount": dash_to_credits!(1200),
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                    "transactionIndex": i,
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

        let pooled = platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::POOLED.into(),
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch withdrawal documents");
        assert_eq!(pooled.len(), 2);
        let queued = platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch withdrawal documents");
        assert_eq!(queued.len(), 1);
        for document in &pooled {
            assert!(document
                .properties()
                .get_u64(withdrawal::properties::TRANSACTION_INDEX)
                .is_ok());
        }
        // Both pooled withdrawals are in flight
        let in_flight = platform
            .drive
            .grove_get_sum_tree_total_value(
                (&get_withdrawal_root_path()).into(),
                &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the in-flight sum");
        assert_eq!(in_flight, dash_to_credits!(2400) as i64);

        // Nothing more fits until Core mines them, whatever the pool says
        platform
            .pool_withdrawals_into_transactions_queue_v2(
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("to pool withdrawal documents into transactions");
        let queued = platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch withdrawal documents");
        assert_eq!(queued.len(), 1);
    }

    #[test]
    fn should_reject_a_credit_pool_reported_at_another_height() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        platform
            .core_rpc
            .expect_get_credit_pool_info()
            .returning(|height| {
                Ok(credit_pool_info(
                    height + 1,
                    dash_to_duffs!(20000),
                    dash_to_duffs!(20000),
                    dash_to_duffs!(4000),
                ))
            });
        let transaction = platform.drive.grove.start_transaction();
        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));
        let document = get_withdrawal_document_fixture(
            &data_contract,
            Identifier::new([1u8; 32]),
            platform_value!({
                "amount": dash_to_credits!(1),
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
        setup_document(
            &platform.drive,
            &document,
            &data_contract,
            data_contract
                .document_type_for_name(withdrawal::NAME)
                .expect("expected to get document type"),
            Some(&transaction),
        );

        assert!(platform
            .pool_withdrawals_into_transactions_queue_v2(
                &BlockInfo {
                    core_height: 960,
                    ..Default::default()
                },
                Some(&transaction),
                platform_version,
            )
            .is_err());
    }
}
