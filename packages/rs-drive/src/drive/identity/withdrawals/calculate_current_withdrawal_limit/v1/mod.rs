use crate::drive::identity::withdrawals::calculate_current_withdrawal_limit::WithdrawalLimitInfo;
use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec,
    get_withdrawal_credit_inflows_sum_tree_path_vec, get_withdrawal_transactions_sum_tree_path_vec,
};
use crate::drive::identity::withdrawals::{
    core_dated_credit_inflow_key, decode_core_dated_credit_inflow_key, DAY_AND_A_HOUR_IN_MS,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::withdrawal::daily_withdrawal_limit::daily_withdrawal_limit;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::{Element, PathQuery, Query, QueryItem, TransactionArg};
use platform_version::version::PlatformVersion;

/// The error messages `sum_expiry_keyed_entries_at_or_after` uses for one tree's entries.
struct ExpiryKeyedEntryErrors {
    not_a_sum_item: &'static str,
    negative: &'static str,
    overflow: &'static str,
}

const CREDIT_INFLOW_ERRORS: ExpiryKeyedEntryErrors = ExpiryKeyedEntryErrors {
    not_a_sum_item: "credit inflow entry is not a sum item",
    negative: "credit inflow entry is negative",
    overflow: "credit inflow entries overflow",
};

const WITHDRAWAL_RESERVATION_ERRORS: ExpiryKeyedEntryErrors = ExpiryKeyedEntryErrors {
    not_a_sum_item: "withdrawal reservation entry is not a sum item",
    negative: "withdrawal reservation entry is negative",
    overflow: "withdrawal reservation entries overflow",
};

impl Drive {
    /// Calculates the current withdrawal limit from the total credits Platform held a day ago
    /// and the credit inflows and withdrawal reservations recorded since that snapshot.
    ///
    /// Version 1 differs from version 0 in the daily maximum and the counted reservations:
    ///
    /// * the base is not the current total credits but the total credits recorded at the latest
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
    ///   bounded cleanup has not deleted yet cannot outlive its 25 hours here. When the
    ///   protocol version caps the base (`max_daily_withdrawal_amount`; protocol version 14
    ///   sets no cap) the inflows ride above the cap: capping the sum would hand the whole
    ///   capped budget back to a deposit-withdraw cycle whenever the base reaches the cap.
    ///   Outflow funded by same-window deposits mirrors Core v24's net credit pool rule;
    /// * the withdrawal reservations are bounded the same way: one pooled at or before the
    ///   snapshot describes an outflow the base already reflects (the history is recorded
    ///   after state transitions executed), so subtracting it again would deny budget the
    ///   guarantee does not require — a deposit-withdraw cycle would stay debited for the
    ///   hour its reservation outlives the snapshot instead of cancelling exactly;
    /// * asset lock inflows are dated by the Core block that mined them, the way Core counts
    ///   them, not by the Platform block that consumed them: they sit in their own tree and
    ///   count while the chain locked height is below the Core height they stop counting at
    ///   (`core_credit_pool_window_min_blocks` after the mining block) and only when recorded
    ///   after the snapshot, for the same reason as the other inflows. An asset lock published
    ///   to Platform long after Core mined it therefore adds nothing. This tree is written from
    ///   protocol version 14 only, the one version that selects this generation.
    ///
    /// The formula stays `daily_maximum - withdrawals_amount`, floored at zero, with both
    /// sides counted over the interval after the snapshot.
    pub(super) fn calculate_current_withdrawal_limit_v1(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<WithdrawalLimitInfo, Error> {
        let recorded_a_day_ago = self.fetch_total_credits_in_platform_a_day_ago(
            block_info.time_ms,
            transaction,
            platform_version,
        )?;

        // Entries of both trees are keyed by their expiration, 25 hours after the recording
        // block, so an entry counts while its key is at or past two bounds: the block time
        // (unexpired — matching the strict cutoff of the cleanup, whose bounded batch may lag
        // behind), and just past the base snapshot plus 25 hours (recorded after the
        // snapshot). An inflow at or before the snapshot is already inside the base — adding
        // it again would let the pool level drop below the guaranteed share of the day-old
        // total — and a reservation at or before it describes an outflow the base already
        // reflects, so subtracting it again would deny budget the guarantee does not require.
        let counted_from = {
            let unexpired = block_info.time_ms;
            match &recorded_a_day_ago {
                Some(recorded) => unexpired.max(
                    recorded
                        .time_ms
                        .saturating_add(DAY_AND_A_HOUR_IN_MS)
                        .saturating_add(1),
                ),
                None => unexpired,
            }
        };

        let credit_inflows_since_the_snapshot = self.sum_expiry_keyed_entries_at_or_after(
            get_withdrawal_credit_inflows_sum_tree_path_vec(),
            counted_from,
            CREDIT_INFLOW_ERRORS,
            transaction,
            platform_version,
        )?;

        let total_credits_a_day_ago = recorded_a_day_ago.map(|recorded| recorded.total_credits);

        // A cap on the base, when the protocol version sets one, applies inside
        // `daily_withdrawal_limit`; the inflows ride on top of the capped base, not under the
        // cap. Capping the sum would discard the inflows exactly when the base reaches the
        // cap — at that point a deposit-withdraw cycle would consume the whole capped budget
        // again (#4471). What Core will mine bounds pooling separately, through the
        // Core-anchored limit.
        let core_dated_credit_inflows_since_the_snapshot = self
            .sum_core_dated_credit_inflows_counting_at(
                block_info.core_height,
                recorded_a_day_ago.as_ref().map(|recorded| recorded.time_ms),
                transaction,
                platform_version,
            )?;

        let daily_maximum = daily_withdrawal_limit(total_credits_a_day_ago, platform_version)?
            .saturating_add(credit_inflows_since_the_snapshot)
            .saturating_add(core_dated_credit_inflows_since_the_snapshot);

        let withdrawals_since_the_snapshot = self.sum_expiry_keyed_entries_at_or_after(
            get_withdrawal_transactions_sum_tree_path_vec(),
            counted_from,
            WITHDRAWAL_RESERVATION_ERRORS,
            transaction,
            platform_version,
        )?;

        Ok(WithdrawalLimitInfo {
            daily_maximum,
            withdrawals_amount: withdrawals_since_the_snapshot,
        })
    }

    /// Sums the Core-dated credit inflows still counting at `core_height` (their key's Core
    /// height is above it) that were recorded after the base snapshot taken at
    /// `snapshot_time_ms` (all of them while no snapshot exists yet). The tree holds the
    /// inflows of one window plus whatever the bounded cleanup has not deleted yet.
    fn sum_core_dated_credit_inflows_counting_at(
        &self,
        core_height: u32,
        snapshot_time_ms: Option<u64>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let Some(first_counting_height) = core_height.checked_add(1) else {
            return Ok(0);
        };

        let path_query = PathQuery::new_unsized(
            get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFrom(
                core_dated_credit_inflow_key(first_counting_height, 0)..,
            )),
        );

        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let mut total: u64 = 0;
        for (key, element) in results.to_key_elements() {
            let (_, recorded_at_time_ms) = decode_core_dated_credit_inflow_key(&key)?;
            if snapshot_time_ms.is_some_and(|snapshot| recorded_at_time_ms <= snapshot) {
                // Minted at or before the snapshot: already inside the base.
                continue;
            }
            let Element::SumItem(amount, _) = element else {
                return Err(Error::Drive(DriveError::CorruptedElementType(
                    "core-dated credit inflow entry is not a sum item",
                )));
            };
            let amount: u64 = amount.try_into().map_err(|_| {
                Error::Drive(DriveError::CriticalCorruptedState(
                    "core-dated credit inflow entry is negative",
                ))
            })?;
            total = total.checked_add(amount).ok_or(Error::Drive(
                DriveError::CriticalCorruptedState("core-dated credit inflow entries overflow"),
            ))?;
        }

        Ok(total)
    }

    /// Sums the sum-item entries of `path` whose expiration key is at or after
    /// `from_time_ms`. Each tree only ever holds the recording blocks of one 25-hour window
    /// plus whatever the bounded cleanup has not deleted yet, so walking the range stays
    /// small; `what` names the entries in errors.
    fn sum_expiry_keyed_entries_at_or_after(
        &self,
        path: Vec<Vec<u8>>,
        from_time_ms: u64,
        what: ExpiryKeyedEntryErrors,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let path_query = PathQuery::new_unsized(
            path,
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
        for result in results.into_iterator() {
            let QueryResultElement::ElementResultItem(element) = result else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "element query returned a non-element result",
                )));
            };
            let Element::SumItem(amount, _) = element else {
                return Err(Error::Drive(DriveError::CorruptedElementType(
                    what.not_a_sum_item,
                )));
            };
            let amount: u64 = amount
                .try_into()
                .map_err(|_| Error::Drive(DriveError::CriticalCorruptedState(what.negative)))?;
            total = total.checked_add(amount).ok_or(Error::Drive(
                DriveError::CriticalCorruptedState(what.overflow),
            ))?;
        }

        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::identity::withdrawals::fetch_total_credits_in_platform_a_day_ago::DAY_IN_MS;

    const HOUR_IN_MS: u64 = DAY_IN_MS / 24;
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

    /// Inflows extend the daily maximum past a capped base: capping the sum instead would hand
    /// the whole capped budget back to a deposit-withdraw cycle whenever the base reaches the
    /// cap — #4471. Protocol version 14 sets no cap, but the rule holds for any that does: at a
    /// 30,000 Dash total a base capped at 4,000 must leave the full 4,000 available to others
    /// after a 4,000 Dash deposit-withdraw cycle.
    #[test]
    fn a_cycle_at_the_capped_base_should_not_consume_the_budget_of_others() {
        let drive = setup_drive_with_initial_state_structure(None);
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .system_limits
            .daily_withdrawal_limit_percent = Some(15);
        platform_version.system_limits.max_daily_withdrawal_amount = Some(dash_to_credits!(4000));
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

        // Platform holds 30,000 Dash: 15% would be 4,500, so the base sits at the 4,000 Dash
        // cap — the network conditions of #4471.
        drive
            .add_to_system_credits(
                dash_to_credits!(30000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to add credits");
        drive
            .record_total_credits_history(&block(t0), 64, Some(&transaction), &platform_version)
            .expect("expected to record");

        let day_one = t0 + DAY_IN_MS;
        assert_eq!(limit(day_one).daily_maximum, dash_to_credits!(4000));

        // The attacker deposits the whole capped budget and withdraws it again.
        drive
            .add_to_system_credits(
                dash_to_credits!(4000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to add the deposit");
        drive
            .record_credit_inflow(
                dash_to_credits!(4000),
                &block(day_one),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to record the inflow");
        let mut drive_operations = vec![];
        drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                vec![(1, vec![0u8; 32])],
                dash_to_credits!(4000),
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
                dash_to_credits!(4000),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to remove the withdrawn credits");

        // The inflow rides above the cap, so the cycle nets out: everyone else still has the
        // full capped budget available.
        let info = limit(day_one);
        assert_eq!(info.daily_maximum, dash_to_credits!(8000));
        assert_eq!(info.withdrawals_amount, dash_to_credits!(4000));
        assert_eq!(info.available(), dash_to_credits!(4000));
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

    /// A reservation whose outflow the day-old base already reflects must not be subtracted
    /// again: once the post-withdrawal total is the snapshot, both the deposit's inflow and
    /// the withdrawal's reservation drop out together and the cycle has cancelled exactly.
    /// Before the fix the reservation kept debiting `available()` for the hour it outlived
    /// the snapshot.
    #[test]
    fn a_reservation_inside_the_day_old_base_should_not_be_subtracted_again() {
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

        // An hour later 500 Dash is deposited, and an hour after that withdrawn again:
        // the withdrawal executes (the total returns to 20,000) and is pooled, reserving
        // 500 Dash until t2 + 25h.
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

        let t2 = t0 + 2 * hour;
        drive
            .remove_from_system_credits(
                dash_to_credits!(500),
                Some(&transaction),
                &platform_version,
            )
            .expect("expected to remove the withdrawn credits");
        let mut drive_operations = vec![];
        drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                vec![(1, vec![0u8; 32])],
                dash_to_credits!(500),
                &mut drive_operations,
                &platform_version,
            )
            .expect("expected to enqueue the withdrawal");
        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block(t2),
                Some(&transaction),
                &platform_version,
                None,
            )
            .expect("expected to apply the pooling operations");
        drive
            .record_total_credits_history(&block(t2), 64, Some(&transaction), &platform_version)
            .expect("expected to record");

        // A millisecond before the withdrawal block becomes the snapshot the base is the
        // 20,500 Dash recorded at the deposit: its inflow is inside that base and the
        // reservation counts against it.
        let info = limit(t2 + DAY_IN_MS - 1);
        assert_eq!(info.daily_maximum, dash_to_credits!(3075));
        assert_eq!(info.withdrawals_amount, dash_to_credits!(500));
        assert_eq!(info.available(), dash_to_credits!(2575));

        // The moment the post-withdrawal 20,000 Dash is the snapshot, the reservation's
        // outflow is inside the base too and both sides of the cycle drop out together:
        // the full 15% of 20,000 is available again, an hour before the reservation expires.
        let info = limit(t2 + DAY_IN_MS);
        assert_eq!(info.daily_maximum, dash_to_credits!(3000));
        assert_eq!(info.withdrawals_amount, 0);
        assert_eq!(info.available(), dash_to_credits!(3000));
    }

    /// An asset lock counts from the Core block that mined it, for
    /// `core_credit_pool_window_min_blocks` (552) Core blocks, the way Core's own limit counts
    /// it, not for a day after the Platform block that consumed it.
    #[test]
    fn an_asset_lock_inflow_should_count_until_a_window_after_the_core_block_that_mined_it() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let t0 = 10 * DAY_IN_MS;
        let t1 = t0 + DAY_IN_MS;
        let block = |time_ms: u64, core_height: u32| BlockInfo {
            time_ms,
            core_height,
            ..Default::default()
        };
        let limit = |time_ms: u64, core_height: u32| {
            drive
                .calculate_current_withdrawal_limit(
                    &block(time_ms, core_height),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the limit")
                .daily_maximum
        };

        drive
            .add_to_system_credits(
                dash_to_credits!(20000),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to add credits");
        drive
            .record_total_credits_history(&block(t0, 400), 64, Some(&transaction), platform_version)
            .expect("expected to record");
        assert_eq!(limit(t1, 1000), dash_to_credits!(3000));

        // Consumed on Platform at Core height 1000, mined by Core at 990: it counts until Core
        // height 990 + 552.
        drive
            .record_asset_lock_credit_inflow(
                [1; 32],
                dash_to_credits!(500),
                Some(990),
                &block(t1, 1000),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the inflow");
        assert_eq!(limit(t1, 1000), dash_to_credits!(3500));
        assert_eq!(limit(t1 + HOUR_IN_MS, 1541), dash_to_credits!(3500));
        assert_eq!(limit(t1 + HOUR_IN_MS, 1542), dash_to_credits!(3000));
    }

    /// The edge case the Core dating closes: an asset lock published to Platform a whole
    /// window after Core mined it sits in the balance Core's limit starts from, so it must not
    /// add budget on Platform's side either.
    #[test]
    fn an_asset_lock_published_a_window_after_core_mined_it_should_add_nothing() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let t0 = 10 * DAY_IN_MS;
        let t1 = t0 + DAY_IN_MS;
        let block = |time_ms: u64, core_height: u32| BlockInfo {
            time_ms,
            core_height,
            ..Default::default()
        };

        drive
            .add_to_system_credits(
                dash_to_credits!(20000),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to add credits");
        drive
            .record_total_credits_history(&block(t0, 400), 64, Some(&transaction), platform_version)
            .expect("expected to record");

        // Mined at Core height 400, consumed at 1000: 400 + 552 is already behind.
        drive
            .record_asset_lock_credit_inflow(
                [1; 32],
                dash_to_credits!(5000),
                Some(400),
                &block(t1, 1000),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the inflow");

        let info = drive
            .calculate_current_withdrawal_limit(
                &block(t1, 1000),
                Some(&transaction),
                platform_version,
            )
            .expect("expected the limit");
        assert_eq!(info.daily_maximum, dash_to_credits!(3000));
    }

    /// An asset lock Platform consumed before Core mined it adds nothing until a Core block
    /// holding it is read, and then counts from that block.
    #[test]
    fn a_pending_asset_lock_inflow_should_count_once_a_core_block_holding_it_is_read() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let t0 = 10 * DAY_IN_MS;
        let t1 = t0 + DAY_IN_MS;
        let block = |time_ms: u64, core_height: u32| BlockInfo {
            time_ms,
            core_height,
            ..Default::default()
        };
        let limit = |time_ms: u64, core_height: u32| {
            drive
                .calculate_current_withdrawal_limit(
                    &block(time_ms, core_height),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the limit")
                .daily_maximum
        };

        drive
            .add_to_system_credits(
                dash_to_credits!(20000),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to add credits");
        drive
            .record_total_credits_history(&block(t0, 400), 64, Some(&transaction), platform_version)
            .expect("expected to record");

        drive
            .record_asset_lock_credit_inflow(
                [7; 32],
                dash_to_credits!(500),
                None,
                &block(t1, 1000),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the inflow");
        assert_eq!(limit(t1, 1000), dash_to_credits!(3000));

        // A Core block that does not hold it changes nothing.
        drive
            .record_core_credit_pool_block(
                1001,
                dash_to_credits!(30000),
                &[[9; 32]],
                &block(t1 + 1, 1001),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the block");
        assert_eq!(limit(t1 + 1, 1001), dash_to_credits!(3000));

        // The block that mined it dates it: it counts until 1002 + 552.
        drive
            .record_core_credit_pool_block(
                1002,
                dash_to_credits!(30500),
                &[[7; 32]],
                &block(t1 + 2, 1002),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the block");
        assert_eq!(limit(t1 + 2, 1002), dash_to_credits!(3500));
        assert_eq!(limit(t1 + HOUR_IN_MS, 1553), dash_to_credits!(3500));
        assert_eq!(limit(t1 + HOUR_IN_MS, 1554), dash_to_credits!(3000));

        // Reading the same block again finds nothing pending: no double count.
        drive
            .record_core_credit_pool_block(
                1002,
                dash_to_credits!(30500),
                &[[7; 32]],
                &block(t1 + 3, 1002),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the block");
        assert_eq!(limit(t1 + 3, 1002), dash_to_credits!(3500));
    }

    /// Like the other inflows, a Core-dated one minted at or before the day-old snapshot is
    /// inside the base and must not count a second time, even while Core still counts it.
    #[test]
    fn a_core_dated_inflow_inside_the_day_old_base_should_not_count_again() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let t0 = 10 * DAY_IN_MS;
        let t1 = t0 + HOUR_IN_MS;
        let block = |time_ms: u64, core_height: u32| BlockInfo {
            time_ms,
            core_height,
            ..Default::default()
        };
        let limit = |time_ms: u64, core_height: u32| {
            drive
                .calculate_current_withdrawal_limit(
                    &block(time_ms, core_height),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the limit")
                .daily_maximum
        };

        drive
            .add_to_system_credits(
                dash_to_credits!(20000),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to add credits");
        drive
            .record_total_credits_history(&block(t0, 100), 64, Some(&transaction), platform_version)
            .expect("expected to record");

        drive
            .add_to_system_credits(dash_to_credits!(500), Some(&transaction), platform_version)
            .expect("expected to add the deposit");
        drive
            .record_asset_lock_credit_inflow(
                [1; 32],
                dash_to_credits!(500),
                Some(120),
                &block(t1, 120),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the inflow");
        drive
            .record_total_credits_history(&block(t1, 120), 64, Some(&transaction), platform_version)
            .expect("expected to record");

        // Before the deposit block is the snapshot the inflow counts: 15% of 20,000 + 500.
        assert_eq!(limit(t1 + 23 * HOUR_IN_MS, 600), dash_to_credits!(3500));
        // Once it is the snapshot the 500 Dash sit inside the 20,500 base; Core still counts
        // it (120 + 552 is ahead), Platform does not count it twice.
        assert_eq!(limit(t1 + DAY_IN_MS, 610), dash_to_credits!(3075));
    }
}
