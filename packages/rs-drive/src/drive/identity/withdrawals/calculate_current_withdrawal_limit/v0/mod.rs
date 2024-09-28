use crate::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path, WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::fee::Credits;
use dpp::withdrawal::daily_withdrawal_limit::daily_withdrawal_limit;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Calculates the current withdrawal limit based on the available credits in the platform
    /// and the amount already withdrawn in the last 24 hours.
    ///
    /// This function considers two main components to calculate the current withdrawal limit:
    /// 1. The total maximum withdrawal allowed in a 24-hour period (`daily_maximum`).
    /// 2. The amount already withdrawn in the last 24 hours (`withdrawal_amount_in_last_day`).
    ///
    /// The formula used is: `daily_maximum - withdrawal_amount_in_last_day`.
    /// If the withdrawal amount in the last 24 hours exceeds the daily maximum, the result will be 0.
    ///
    /// # Parameters
    ///
    /// * `transaction`: The transaction context to use for querying data.
    /// * `platform_version`: The version of the platform being used, containing relevant configuration details.
    ///
    /// # Returns
    ///
    /// * `Ok(Credits)`: The calculated current withdrawal limit, which is the maximum amount that can still be withdrawn in the current 24-hour window.
    /// * `Err(Error)`: Returns an error if there was an issue retrieving the total credits, the daily maximum, or the already withdrawn amount.
    ///
    /// # Errors
    ///
    /// * `Error::Drive(DriveError::CriticalCorruptedState)`:
    ///   - If the total credits in the platform cannot be found, indicating a critical state corruption.
    ///   - If the withdrawal amount in the last 24 hours is negative, which should not happen.
    pub(super) fn calculate_current_withdrawal_limit_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        // The current withdrawal limit has two components
        // 1. The total maximum that we are allowed to do in 24 hours
        // 2. The amount that we have already withdrawn in the last 24 hours
        let mut drive_operations = vec![];

        let path_holding_total_credits = misc_path();
        let total_credits_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_total_credits).into(),
                TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits not found in Platform",
            )))?;

        // Let's get the amount that we are allowed to get in the last 24 hours.
        let daily_maximum = daily_withdrawal_limit(total_credits_in_platform, platform_version)?;

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

        Ok(daily_maximum.saturating_sub(withdrawal_amount_in_last_day))
    }
}
