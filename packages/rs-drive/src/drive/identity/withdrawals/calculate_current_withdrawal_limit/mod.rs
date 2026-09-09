use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

/// Daily withdrawal limit information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WithdrawalLimitInfo {
    /// The total maximum withdrawal amount allowed in a 24-hour period.
    pub daily_maximum: Credits,
    /// The amount already withdrawn in the last 24 hours.
    pub withdrawals_amount: Credits,
}

impl WithdrawalLimitInfo {
    /// Calculates the available credits to withdraw
    pub fn available(&self) -> Credits {
        self.daily_maximum.saturating_sub(self.withdrawals_amount)
    }
}

impl Drive {
    /// Calculates the current withdrawal limit based on the total credits available in the platform
    /// and the amount already withdrawn in the last 24 hours, using the appropriate version-specific logic.
    ///
    /// This function selects the version-specific implementation based on the provided `platform_version`:
    /// version 0 derives the daily maximum from the current total credits in Platform, version 1
    /// from the total credits Platform held a day before `block_info.time_ms`.
    ///
    /// # Parameters
    ///
    /// * `block_info`: The block the limit is calculated for.
    /// * `transaction`: The transaction context used for querying data.
    /// * `platform_version`: The version of the platform being used, which contains configuration details and version-specific methods.
    ///
    /// # Returns
    ///
    /// * `Ok(Credits)`: The calculated current withdrawal limit, representing the maximum amount that can still be withdrawn in the current 24-hour window.
    /// * `Err(Error)`: Returns an error if the version specified in `platform_version` is not supported or if there is an issue in the version-specific calculation.
    ///
    /// # Errors
    ///
    /// * `Error::Drive(DriveError::UnknownVersionMismatch)`:
    ///   - If the platform version provided does not match any known versions supported by this function.
    ///
    /// * `Error`: Any error propagated from the version-specific implementation, such as issues in retrieving data or calculating the withdrawal limit.
    pub fn calculate_current_withdrawal_limit(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<WithdrawalLimitInfo, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .calculate_current_withdrawal_limit
        {
            0 => self.calculate_current_withdrawal_limit_v0(transaction, platform_version),
            1 => self.calculate_current_withdrawal_limit_v1(
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "calculate_current_withdrawal_limit".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
