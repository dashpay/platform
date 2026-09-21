use crate::drive::identity::withdrawals::calculate_current_withdrawal_limit::{
    CoreCreditPoolSnapshot, WithdrawalLimitInfo,
};
use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path, WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::withdrawal::daily_withdrawal_limit::daily_withdrawal_limit;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Mirrors Core's v24 credit pool rule with Platform's smaller share.
    ///
    /// Core admits asset unlocks in a block as long as the pool does not end it below its
    /// balance one window earlier minus an allowed drop (20% of that balance, at least
    /// 2000 Dash). Platform applies the same shape to the same two balances, read from
    /// Core at the chain locked height, with its own share (`daily_withdrawal_limit`:
    /// 15%, at least one maximal withdrawal), and never
    /// goes above Core's own limit for the next block. What is already pooled and not yet
    /// mined counts against it, so the sum of every unlock Platform has outstanding stays
    /// within what Core mines. Deposits and rewards inside the window raise the balance
    /// and are withdrawable again on both chains, without any inflow bookkeeping.
    ///
    /// Core re-evaluates at the tip that mines the unlock, a few blocks after the chain
    /// locked height; the gap between the two shares absorbs what the pool moved in between.
    /// Until Core's v24 activates, Core's limit is the flat 2000 Dash per block of v22, so the
    /// `current_limit` bound keeps Platform at most 2000 Dash in flight: stricter, never looser.
    pub(super) fn calculate_current_withdrawal_limit_v1(
        &self,
        core_credit_pool: &CoreCreditPoolSnapshot,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<WithdrawalLimitInfo, Error> {
        let CoreCreditPoolSnapshot {
            balance,
            window_start_balance,
            current_limit,
        } = *core_credit_pool;

        let allowed_drop = daily_withdrawal_limit(Some(window_start_balance), platform_version)?;

        // The drop the pool already took inside the window comes out of the allowed drop;
        // what it gained is withdrawable on top. Core computes its own limit the same way.
        let platform_limit = if balance >= window_start_balance {
            allowed_drop.saturating_add(balance - window_start_balance)
        } else {
            allowed_drop.saturating_sub(window_start_balance - balance)
        };

        let daily_maximum = platform_limit.min(current_limit);

        let in_flight: u64 = self
            .grove_get_sum_tree_total_value(
                (&get_withdrawal_root_path()).into(),
                &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut vec![],
                &platform_version.drive,
            )?
            .try_into()
            .map_err(|_| {
                Error::Drive(DriveError::CriticalCorruptedState(
                    "the amount of withdrawals in flight is negative",
                ))
            })?;

        Ok(WithdrawalLimitInfo {
            daily_maximum,
            withdrawals_amount: in_flight,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::identity::withdrawals::calculate_current_withdrawal_limit::CoreCreditPoolSnapshot;
    use crate::util::batch::drive_op_batch::WithdrawalOperationType;
    use crate::util::batch::DriveOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::dash_to_credits;
    use dpp::version::PlatformVersion;

    fn pool(balance: u64, window_start_balance: u64, current_limit: u64) -> CoreCreditPoolSnapshot {
        CoreCreditPoolSnapshot {
            balance,
            window_start_balance,
            current_limit,
        }
    }

    /// Core's v24 rule for the same balances, to feed `current_limit` with what Core would say
    fn core_limit(balance: u64, window_start_balance: u64) -> u64 {
        let allowed_drop = (window_start_balance * 20 / 100).max(dash_to_credits!(2000));
        (allowed_drop + balance)
            .saturating_sub(window_start_balance)
            .min(balance)
    }

    #[test]
    fn should_take_the_platform_share_of_the_window_start_balance_net_of_the_windows_drop() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();
        let limit = |balance: u64, window_start_balance: u64| {
            drive
                .calculate_current_withdrawal_limit(
                    Some(&pool(
                        balance,
                        window_start_balance,
                        core_limit(balance, window_start_balance),
                    )),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the limit")
        };

        // 15% of the balance one window ago, Core allows 20%
        let info = limit(dash_to_credits!(20000), dash_to_credits!(20000));
        assert_eq!(info.daily_maximum, dash_to_credits!(3000));
        assert_eq!(info.withdrawals_amount, 0);
        assert_eq!(info.available(), dash_to_credits!(3000));
        // One maximal withdrawal at least; Core's floor is 2000
        assert_eq!(
            limit(dash_to_credits!(2000), dash_to_credits!(2000)).daily_maximum,
            dash_to_credits!(500)
        );
        // No absolute cap: the share scales with the pool
        assert_eq!(
            limit(dash_to_credits!(40000), dash_to_credits!(40000)).daily_maximum,
            dash_to_credits!(6000)
        );
        // What the pool already lost inside the window comes out of the share
        assert_eq!(
            limit(dash_to_credits!(19000), dash_to_credits!(20000)).daily_maximum,
            dash_to_credits!(2000)
        );
        assert_eq!(
            limit(dash_to_credits!(16000), dash_to_credits!(20000)).daily_maximum,
            0
        );
        // What it gained is withdrawable on top: a deposit consumes nobody's budget
        assert_eq!(
            limit(dash_to_credits!(21000), dash_to_credits!(20000)).daily_maximum,
            dash_to_credits!(4000)
        );
        // No pool one window ago: the whole balance is a deposit inside the window
        assert_eq!(
            limit(dash_to_credits!(123), 0).daily_maximum,
            dash_to_credits!(123)
        );
    }

    #[test]
    fn should_never_exceed_what_core_admits_in_the_next_block() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let info = drive
            .calculate_current_withdrawal_limit(
                Some(&pool(
                    dash_to_credits!(20000),
                    dash_to_credits!(20000),
                    dash_to_credits!(1000),
                )),
                Some(&transaction),
                platform_version,
            )
            .expect("expected the limit");
        assert_eq!(info.daily_maximum, dash_to_credits!(1000));
    }

    #[test]
    fn should_count_the_withdrawals_in_flight_until_they_are_released() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();
        let core_pool = pool(
            dash_to_credits!(20000),
            dash_to_credits!(20000),
            dash_to_credits!(4000),
        );
        let limit = || {
            drive
                .calculate_current_withdrawal_limit(
                    Some(&core_pool),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the limit")
        };

        drive
            .apply_drive_operations(
                vec![DriveOperation::WithdrawalOperation(
                    WithdrawalOperationType::ReserveInFlightWithdrawals {
                        amounts: vec![(1, dash_to_credits!(1000)), (2, dash_to_credits!(500))],
                    },
                )],
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to reserve");
        let info = limit();
        assert_eq!(info.daily_maximum, dash_to_credits!(3000));
        assert_eq!(info.withdrawals_amount, dash_to_credits!(1500));
        assert_eq!(info.available(), dash_to_credits!(1500));

        // Core mined the first one (an unknown index is ignored)
        drive
            .apply_drive_operations(
                vec![DriveOperation::WithdrawalOperation(
                    WithdrawalOperationType::ReleaseInFlightWithdrawals {
                        indexes: vec![1, 7],
                    },
                )],
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to release");
        let info = limit();
        assert_eq!(info.withdrawals_amount, dash_to_credits!(500));
        assert_eq!(info.available(), dash_to_credits!(2500));
    }

    #[test]
    fn should_require_cores_credit_pool() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        assert!(drive
            .calculate_current_withdrawal_limit(None, Some(&transaction), platform_version)
            .is_err());
    }
}
