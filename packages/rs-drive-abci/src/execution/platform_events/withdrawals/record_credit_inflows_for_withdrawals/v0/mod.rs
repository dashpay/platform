use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Delegates to `Drive::record_credit_inflow`, which records nothing for a zero amount.
    pub(super) fn record_credit_inflows_for_withdrawals_v0(
        &self,
        credit_mints: Credits,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive
            .record_credit_inflow(
                credit_mints,
                block_info,
                Some(transaction),
                platform_version,
            )
            .map_err(Error::Drive)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::dash_to_credits;
    use dpp::version::PlatformVersion;
    use drive::drive::identity::withdrawals::paths::{
        get_withdrawal_root_path, WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
    };
    use drive::util::grove_operations::DirectQueryType;

    /// The event records the block's mints in the credit inflows sum tree, accumulating
    /// within a block time, and records nothing for a block that minted nothing.
    #[test]
    fn should_record_the_blocks_mints_and_skip_zero() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 10,
            epoch: Epoch::default(),
        };

        let inflows = |transaction: &_| {
            platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    Some(transaction),
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected the inflows sum")
        };

        platform
            .record_credit_inflows_for_withdrawals(0, &block_info, &transaction, platform_version)
            .expect("expected to record nothing");
        assert_eq!(inflows(&transaction), 0);

        platform
            .record_credit_inflows_for_withdrawals(
                dash_to_credits!(3),
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to record");
        platform
            .record_credit_inflows_for_withdrawals(
                dash_to_credits!(2),
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to add to the same entry");

        assert_eq!(inflows(&transaction), dash_to_credits!(5) as i64);
    }

    /// Before protocol version 14 the version slot is `None` and the event does nothing.
    #[test]
    fn should_do_nothing_before_the_feature_exists() {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version =
            PlatformVersion::get(13).expect("expected to get platform version 13");
        let transaction = platform.drive.grove.start_transaction();

        platform
            .record_credit_inflows_for_withdrawals(
                dash_to_credits!(3),
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected the event to be a no-op before v14");
    }
}
