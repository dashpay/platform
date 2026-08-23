use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Delegates to `Drive::record_total_credits_history`, bounding the per-block prune by
    /// `withdrawal_constants.total_credits_history_prune_limit`.
    pub(super) fn record_total_credits_history_for_withdrawals_v0(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive
            .record_total_credits_history(
                block_info,
                platform_version
                    .drive_abci
                    .withdrawal_constants
                    .total_credits_history_prune_limit,
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
    use dpp::dash_to_credits;
    use dpp::version::PlatformVersion;
    use drive::drive::identity::withdrawals::fetch_total_credits_in_platform_a_day_ago::{
        RecordedTotalCredits, DAY_IN_MS,
    };

    /// Every block leaves an entry holding the total credits in Platform at that block.
    #[test]
    fn v0_records_the_total_credits_for_the_block() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();

        platform
            .drive
            .add_to_system_credits(
                dash_to_credits!(30000),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to add credits");

        let block_info = BlockInfo {
            time_ms: 1_000_000_000,
            height: 100,
            ..Default::default()
        };

        platform
            .record_total_credits_history_for_withdrawals_v0(
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to record the total credits");

        // Readable as the reference a day later
        assert_eq!(
            platform
                .drive
                .fetch_total_credits_in_platform_a_day_ago(
                    block_info.time_ms + DAY_IN_MS,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to fetch"),
            Some(RecordedTotalCredits {
                time_ms: block_info.time_ms,
                total_credits: dash_to_credits!(30000),
            })
        );
    }

    /// The dispatcher is a no-op for protocol versions without the history (slot is `None`).
    #[test]
    fn dispatcher_is_noop_when_the_slot_is_none() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive_abci
            .methods
            .withdrawals
            .record_total_credits_history_for_withdrawals = None;
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1_000_000_000,
            ..Default::default()
        };

        platform
            .record_total_credits_history_for_withdrawals(
                &block_info,
                &transaction,
                &platform_version,
            )
            .expect("expected a no-op");

        assert_eq!(
            platform
                .drive
                .fetch_total_credits_in_platform_a_day_ago(
                    block_info.time_ms + DAY_IN_MS,
                    Some(&transaction),
                    &platform_version,
                )
                .expect("expected to fetch"),
            None
        );
    }
}
