use crate::drive::identity::withdrawals::calculate_current_withdrawal_limit::WithdrawalLimitInfo;
use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path, WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
    WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
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
    /// Calculates the current withdrawal limit from the total credits Platform held a day ago,
    /// the credit inflows of the last 25 hours and the amount already withdrawn in the last
    /// 24 hours.
    ///
    /// Version 1 differs from version 0 in the daily maximum:
    ///
    /// * its base is not the current total credits but the total credits recorded at the latest
    ///   block at least 24 hours before `block_info` (see
    ///   `fetch_total_credits_in_platform_a_day_ago`), so a sudden jump in the total credits
    ///   does not raise the limit for a day. While no such entry exists (the history is younger
    ///   than a day) the limit method receives `None` and applies its bootstrap rule;
    /// * the credit inflows recorded in the last 25 hours (asset locks, epoch Core rewards —
    ///   money that verifiably entered Platform, so a Platform minting bug cannot forge it) are
    ///   added on top, making the limit one on net outflow instead of gross: a deposit and the
    ///   withdrawal it funds cancel out and consume none of the budget of other users. The sum
    ///   stays capped by `max_daily_withdrawal_amount`, the unlock capacity Core mines per day,
    ///   which is a constraint on gross outflow that inflows cannot lift.
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

        let credit_inflows_in_last_day: u64 = self
            .grove_get_sum_tree_total_value(
                (&get_withdrawal_root_path()).into(),
                &WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .try_into()
            .map_err(|_| {
                Error::Drive(DriveError::CriticalCorruptedState(
                    "Credit inflows in last day are negative",
                ))
            })?;

        let daily_maximum = {
            let uncapped = daily_withdrawal_limit(total_credits_a_day_ago, platform_version)?
                .saturating_add(credit_inflows_in_last_day);
            match platform_version.system_limits.max_daily_withdrawal_amount {
                Some(max_daily_withdrawal_amount) => uncapped.min(max_daily_withdrawal_amount),
                None => uncapped,
            }
        };

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
    use crate::util::batch::{DriveOperation, SystemOperationType};
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
                dash_to_credits!(20000),
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

        // The total jumps to 24000 Dash, but a day after the first record the limit derives
        // from the 20000 Dash recorded then.
        drive
            .add_to_system_credits(
                dash_to_credits!(4000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to add credits");
        assert_eq!(limit(t0 + DAY_IN_MS).daily_maximum, dash_to_credits!(3000));

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
            dash_to_credits!(3600)
        );
    }

    /// The regression guard for the gross-accounting starvation of issue #4471: a
    /// deposit -> withdraw cycle of one user's own coins must not consume the withdrawal
    /// budget of everyone else. The deposit lands through the production mint operation
    /// (`SystemOperationType::AddToSystemCredits`, what every asset-lock funded transition
    /// emits), which records it in the credit inflows sum tree; the limit adds those inflows
    /// to the daily maximum, so the cycle nets out to the untouched base.
    #[test]
    fn deposit_then_withdraw_cycling_should_not_consume_the_budget_of_others() {
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

        // Platform holds 10,000 Dash. The direct `add_to_system_credits` call is state setup,
        // not a minting operation, so it records no inflow.
        drive
            .add_to_system_credits(
                dash_to_credits!(10000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to add credits");
        drive
            .record_total_credits_history(&block(t0), 64, Some(&transaction), &platform_version)
            .expect("expected to record");

        // A day later the base is 15% of 10,000 = 1,500 Dash and no inflows are recorded.
        let day_one = t0 + DAY_IN_MS;
        assert_eq!(limit(day_one).daily_maximum, dash_to_credits!(1500));

        // The user deposits 1,000 Dash of their own money: the production mint operation
        // records the inflow, which extends the daily maximum by the same amount.
        drive
            .apply_drive_operations(
                vec![DriveOperation::SystemOperation(
                    SystemOperationType::AddToSystemCredits {
                        amount: dash_to_credits!(1000),
                    },
                )],
                true,
                &block(day_one),
                Some(&transaction),
                &platform_version,
                None,
            )
            .expect("expected to apply the deposit");
        assert_eq!(limit(day_one).daily_maximum, dash_to_credits!(2500));

        // ... and withdraws the same 1,000 Dash again: pooling reserves the amount and the
        // executed withdrawal takes the credits back out of Platform.
        let mut drive_operations = vec![];
        drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                vec![(1, vec![0u8; 32])],
                dash_to_credits!(1000),
                &mut drive_operations,
                &platform_version,
            )
            .expect("expected to enqueue the withdrawal");
        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block(day_one),
                Some(&transaction),
                &platform_version,
                None,
            )
            .expect("expected to apply the pooling operations");
        drive
            .remove_from_system_credits(
                dash_to_credits!(1000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to remove the withdrawn credits");

        // The cycle nets out: everyone else still has the full 1,500 Dash base available.
        let info = limit(day_one);
        assert_eq!(info.daily_maximum, dash_to_credits!(2500));
        assert_eq!(info.withdrawals_amount, dash_to_credits!(1000));
        assert_eq!(info.available(), dash_to_credits!(1500));
    }

    /// Inflows extend the daily maximum only up to `max_daily_withdrawal_amount`: Core mines a
    /// bounded amount of credit pool unlocks per day, and that is a constraint on gross
    /// outflow that money entering Platform cannot lift.
    #[test]
    fn credit_inflows_should_not_lift_the_daily_maximum_above_cores_capacity() {
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

        // Platform holds 20,000 Dash, so a day later the base is 15% = 3,000 Dash.
        drive
            .add_to_system_credits(
                dash_to_credits!(20000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to add credits");
        drive
            .record_total_credits_history(&block(t0), 64, Some(&transaction), &platform_version)
            .expect("expected to record");

        // A 5,000 Dash deposit would allow 8,000 Dash of gross outflow; the cap holds it at
        // Core's 4,000 Dash of unlock capacity per day.
        let day_one = t0 + DAY_IN_MS;
        drive
            .apply_drive_operations(
                vec![DriveOperation::SystemOperation(
                    SystemOperationType::AddToSystemCredits {
                        amount: dash_to_credits!(5000),
                    },
                )],
                true,
                &block(day_one),
                Some(&transaction),
                &platform_version,
                None,
            )
            .expect("expected to apply the deposit");

        let info = drive
            .calculate_current_withdrawal_limit(
                &block(day_one),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected the limit");
        assert_eq!(info.daily_maximum, dash_to_credits!(4000));
    }
}
