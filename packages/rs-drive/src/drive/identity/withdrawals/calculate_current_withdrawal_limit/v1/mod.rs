use crate::drive::identity::withdrawals::calculate_current_withdrawal_limit::WithdrawalLimitInfo;
use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_credit_inflows_sum_tree_path_vec, get_withdrawal_root_path,
    WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use crate::drive::identity::withdrawals::DAY_AND_A_HOUR_IN_MS;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::block::block_info::BlockInfo;
use dpp::withdrawal::daily_withdrawal_limit::daily_withdrawal_limit;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, QueryItem, TransactionArg};
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
    /// * the credit inflows recorded since that base snapshot (asset locks, epoch Core
    ///   rewards — money that verifiably entered Platform, so a Platform minting bug cannot
    ///   forge it) are added on top, making the limit one on net outflow instead of gross: a
    ///   deposit and the withdrawal it funds cancel out and consume none of the budget of
    ///   other users. Only inflows younger than the snapshot count — an older one is already
    ///   inside the base, and adding it again would allow the pool level to drop below the
    ///   guaranteed share of the day-old total — and only unexpired ones, so an entry the
    ///   bounded cleanup has not deleted yet cannot outlive its 25 hours here. The sum stays
    ///   capped by `max_daily_withdrawal_amount`, the unlock capacity Core mines per day,
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

        let recorded_a_day_ago = self.fetch_total_credits_in_platform_a_day_ago(
            block_info.time_ms,
            transaction,
            platform_version,
        )?;

        // Inflow entries are keyed by their expiration, 25 hours after the mint, so an inflow
        // counts while its key is at or past two bounds: the block time (unexpired — matching
        // the strict cutoff of the cleanup, whose bounded batch may lag behind), and just past
        // the base snapshot plus 25 hours (minted after the snapshot: anything at or before it
        // is already inside the base, and counting it twice would let the pool level drop
        // below the guaranteed share of the day-old total).
        let unexpired = block_info.time_ms;
        let credit_inflows_since_the_snapshot = match &recorded_a_day_ago {
            Some(recorded) => self.sum_credit_inflows_expiring_at_or_after(
                unexpired.max(
                    recorded
                        .time_ms
                        .saturating_add(DAY_AND_A_HOUR_IN_MS)
                        .saturating_add(1),
                ),
                transaction,
                platform_version,
            )?,
            None => self.sum_credit_inflows_expiring_at_or_after(
                unexpired,
                transaction,
                platform_version,
            )?,
        };

        let total_credits_a_day_ago = recorded_a_day_ago.map(|recorded| recorded.total_credits);

        let daily_maximum = {
            let uncapped = daily_withdrawal_limit(total_credits_a_day_ago, platform_version)?
                .saturating_add(credit_inflows_since_the_snapshot);
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

    /// Sums the credit inflow entries whose expiration key is at or after `from_time_ms`. The
    /// tree only ever holds the mint blocks of one 25-hour window plus whatever the bounded
    /// cleanup has not deleted yet, so walking the range stays small.
    fn sum_credit_inflows_expiring_at_or_after(
        &self,
        from_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let path_query = PathQuery::new_unsized(
            get_withdrawal_credit_inflows_sum_tree_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFrom(
                from_time_ms.to_be_bytes().to_vec()..,
            )),
        );

        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryElementResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let mut total: u64 = 0;
        for element in results.to_elements() {
            let Element::SumItem(amount, _) = element else {
                return Err(Error::Drive(DriveError::CorruptedElementType(
                    "credit inflow entry is not a sum item",
                )));
            };
            let amount: u64 = amount.try_into().map_err(|_| {
                Error::Drive(DriveError::CriticalCorruptedState(
                    "credit inflow entry is negative",
                ))
            })?;
            total = total.checked_add(amount).ok_or(Error::Drive(
                DriveError::CriticalCorruptedState("credit inflows overflow"),
            ))?;
        }

        Ok(total)
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
    /// budget of everyone else. The deposit lands as a recorded credit inflow
    /// (recorded per block by `record_credit_inflow`, fed by the mints every asset-lock
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

        // The user deposits 1,000 Dash of their own money; the block records the mint as a
        // credit inflow, which extends the daily maximum by the same amount.
        drive
            .add_to_system_credits(
                dash_to_credits!(1000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to add the deposit");
        drive
            .record_credit_inflow(
                dash_to_credits!(1000),
                &block(day_one),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to record the inflow");
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
            .add_to_system_credits(
                dash_to_credits!(5000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to add the deposit");
        drive
            .record_credit_inflow(
                dash_to_credits!(5000),
                &block(day_one),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to record the inflow");

        let info = drive
            .calculate_current_withdrawal_limit(
                &block(day_one),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected the limit");
        assert_eq!(info.daily_maximum, dash_to_credits!(4000));
    }

    /// An inflow that is already part of the day-old base must not extend the daily maximum a
    /// second time: while the deposit block is younger than a day the inflow counts against
    /// the older snapshot, and the moment the deposit block itself becomes the snapshot the
    /// inflow is inside the base and drops out of the sum.
    #[test]
    fn an_inflow_inside_the_day_old_base_should_not_count_again() {
        let drive = setup_drive_with_initial_state_structure(None);
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .system_limits
            .daily_withdrawal_limit_percent = Some(15);
        let transaction = drive.grove.start_transaction();

        let hour = DAY_IN_MS / 24;
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
        drive
            .record_total_credits_history(&block(t0), 64, Some(&transaction), &platform_version)
            .expect("expected to record");

        // An hour later 500 Dash is deposited and the new total of 20,500 recorded.
        let t1 = t0 + hour;
        drive
            .add_to_system_credits(dash_to_credits!(500), Some(&transaction), &platform_version)
            .expect("expected to add the deposit");
        drive
            .record_credit_inflow(
                dash_to_credits!(500),
                &block(t1),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to record the inflow");
        drive
            .record_total_credits_history(&block(t1), 64, Some(&transaction), &platform_version)
            .expect("expected to record");

        // 23 hours after the deposit the base is still the 20,000 Dash snapshot from before
        // it, so the inflow counts: 15% of 20,000 + 500.
        assert_eq!(limit(t1 + 23 * hour).daily_maximum, dash_to_credits!(3500));

        // An hour later the deposit block is the base snapshot: the 500 Dash sits inside the
        // 20,500 and only the percentage applies. Counting it again would have allowed
        // 15% of 20,500 + 500 and let the pool level drop below 85% of the day-old total.
        assert_eq!(limit(t1 + DAY_IN_MS).daily_maximum, dash_to_credits!(3075));
    }

    /// An inflow entry the bounded per-block cleanup has not deleted yet must still stop
    /// counting the moment it expires, mirroring the strict cutoff the cleanup uses.
    #[test]
    fn an_expired_but_not_yet_pruned_inflow_should_not_count() {
        let drive = setup_drive_with_initial_state_structure(None);
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .system_limits
            .daily_withdrawal_limit_percent = Some(15);
        let transaction = drive.grove.start_transaction();

        let hour = DAY_IN_MS / 24;
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
        drive
            .record_total_credits_history(&block(t0), 64, Some(&transaction), &platform_version)
            .expect("expected to record");

        // A day later 500 Dash is deposited; its inflow entry expires 25 hours after that.
        let t1 = t0 + DAY_IN_MS;
        drive
            .add_to_system_credits(dash_to_credits!(500), Some(&transaction), &platform_version)
            .expect("expected to add the deposit");
        drive
            .record_credit_inflow(
                dash_to_credits!(500),
                &block(t1),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to record the inflow");

        // At the exact expiration block the entry still counts, exactly like a reservation
        // with that key would still be held: the cleanup deletes strictly older keys only.
        assert_eq!(limit(t1 + 25 * hour).daily_maximum, dash_to_credits!(3500));

        // A millisecond past expiration it no longer counts, even though no cleanup ran and
        // the entry is still in the tree.
        assert_eq!(
            limit(t1 + 25 * hour + 1).daily_maximum,
            dash_to_credits!(3000)
        );
    }
}
