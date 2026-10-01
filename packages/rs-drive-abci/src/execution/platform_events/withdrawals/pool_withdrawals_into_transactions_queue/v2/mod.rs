use dpp::block::block_info::BlockInfo;
use metrics::gauge;

use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

use crate::metrics::{
    GAUGE_CREDIT_WITHDRAWAL_LIMIT_AVAILABLE, GAUGE_CREDIT_WITHDRAWAL_LIMIT_CORE_AVAILABLE,
    GAUGE_CREDIT_WITHDRAWAL_LIMIT_TOTAL,
};
use crate::{error::Error, platform_types::platform::Platform, rpc::core::CoreRPCLike};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Pool withdrawal documents into transactions.
    ///
    /// Version 2 differs from version 1 only in the amount it pools up to: the smaller of the
    /// daily withdrawal limit and the Core-anchored limit, a stricter copy of Core's own asset
    /// unlock rule read from Core's credit pool balances. Platform's own accounting can grant
    /// more than Core will mine (an asset lock published to Platform after Core mined it, the
    /// epoch Core rewards minted in one block); an unlock over Core's limit waits unmined,
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
        // queued: their credit pool balances feed the Core-anchored limit below, and a long
        // jump of the chain locked height is read over several blocks.
        self.scan_core_blocks_for_withdrawals(block_info, transaction, platform_version)?;

        self.pool_withdrawals_up_to_limit_v1(
            block_info,
            transaction,
            platform_version,
            |available, daily_maximum| {
                let core_anchored_withdrawal_limit = self
                    .calculate_core_anchored_withdrawal_limit(
                        block_info,
                        transaction,
                        platform_version,
                    )?;

                tracing::trace!(
                    core_anchored_withdrawal_limit,
                    "Calculated Core-anchored withdrawal limit"
                );

                let current_withdrawal_limit = available.min(core_anchored_withdrawal_limit);

                // Store prometheus metrics: what may be pooled, as in version 1, and the Core
                // side on its own.
                gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_AVAILABLE)
                    .set(current_withdrawal_limit as f64);
                gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_TOTAL).set(daily_maximum as f64);
                gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_CORE_AVAILABLE)
                    .set(core_anchored_withdrawal_limit as f64);

                Ok(current_withdrawal_limit)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::SystemDataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::system_data_contracts::withdrawals_contract;
    use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
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
        core_rpc
            .expect_get_credit_pool_balance()
            .returning(move |_| Ok(pool_duffs));
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
