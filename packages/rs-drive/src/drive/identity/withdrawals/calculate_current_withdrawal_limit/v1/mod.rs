use crate::drive::identity::withdrawals::calculate_current_withdrawal_limit::WithdrawalLimitInfo;
use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path, WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::block::block_info::BlockInfo;
use dpp::withdrawal::daily_withdrawal_limit::daily_withdrawal_limit;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Calculates the current withdrawal limit from the total credits Platform held a day ago
    /// and the amount already withdrawn in the last 24 hours.
    ///
    /// Version 1 differs from version 0 only in the base of the daily maximum: instead of the
    /// current total credits it hands `daily_withdrawal_limit` the total credits recorded at the
    /// latest block at least 24 hours before `block_info` (see
    /// `fetch_total_credits_in_platform_a_day_ago`), so a sudden jump in the total credits does
    /// not raise the limit for a day. While no such entry exists (the history is younger than a
    /// day) the limit method receives `None` and applies its bootstrap rule.
    ///
    /// The formula stays `daily_maximum - withdrawal_amount_in_last_day`, floored at zero.
    pub(super) fn calculate_current_withdrawal_limit_v1(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<WithdrawalLimitInfo, Error> {
        let mut drive_operations = vec![];

        let total_credits_a_day_ago = self
            .fetch_total_credits_in_platform_a_day_ago(
                block_info.time_ms,
                transaction,
                platform_version,
            )?
            .map(|recorded| recorded.total_credits);

        let daily_maximum = daily_withdrawal_limit(total_credits_a_day_ago, platform_version)?;

        let withdrawal_amount_in_last_day: u64 = self
            .grove_get_sum_tree_total_value(
                (&get_withdrawal_root_path()).into(),
                &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .try_into()
            .map_err(|_| {
                Error::Drive(DriveError::CriticalCorruptedState(
                    "Withdrawal amount in last day is negative",
                ))
            })?;

        Ok(WithdrawalLimitInfo {
            daily_maximum,
            withdrawals_amount: withdrawal_amount_in_last_day,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::identity::withdrawals::fetch_total_credits_in_platform_a_day_ago::DAY_IN_MS;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::dash_to_credits;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_use_the_total_credits_a_day_ago_once_known_and_the_flat_limit_before() {
        let drive = setup_drive_with_initial_state_structure(None);
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .system_limits
            .daily_withdrawal_limit_percent = Some(15);
        let transaction = drive.grove.start_transaction();

        let t0 = 10 * DAY_IN_MS;
        let block = |time_ms: u64| BlockInfo {
            time_ms,
            ..Default::default()
        };
        let limit = |time_ms: u64| {
            drive
                .calculate_current_withdrawal_limit(
                    &block(time_ms),
                    Some(&transaction),
                    &platform_version,
                )
                .expect("expected the limit")
        };

        drive
            .add_to_system_credits(
                dash_to_credits!(30000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to add credits");

        // Nothing recorded yet: the flat 2000 Dash of the previous rule applies.
        let info = limit(t0);
        assert_eq!(info.daily_maximum, dash_to_credits!(2000));
        assert_eq!(info.withdrawals_amount, 0);
        assert_eq!(info.available(), dash_to_credits!(2000));

        drive
            .record_total_credits_history(&block(t0), 64, Some(&transaction), &platform_version)
            .expect("expected to record");

        // Recorded, but younger than a day: still the flat limit.
        assert_eq!(
            limit(t0 + DAY_IN_MS - 1).daily_maximum,
            dash_to_credits!(2000)
        );

        // The total jumps to 40000 Dash, but a day after the first record the limit derives
        // from the 30000 Dash recorded then.
        drive
            .add_to_system_credits(
                dash_to_credits!(10000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to add credits");
        assert_eq!(limit(t0 + DAY_IN_MS).daily_maximum, dash_to_credits!(4500));

        // Once the larger total is a day old it becomes the base.
        drive
            .record_total_credits_history(
                &block(t0 + DAY_IN_MS),
                64,
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to record");
        assert_eq!(
            limit(t0 + 2 * DAY_IN_MS).daily_maximum,
            dash_to_credits!(6000)
        );
    }
}
