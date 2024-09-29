use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;

impl Drive {
    /// Calculates the current withdrawal limit based on the total credits available in the platform
    /// and the amount already withdrawn in the last 24 hours, using the appropriate version-specific logic.
    ///
    /// This function selects the version-specific implementation based on the provided `platform_version`.
    /// It currently supports only version 0 (`calculate_current_withdrawal_limit_v0`).
    ///
    /// # Parameters
    ///
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
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .calculate_current_withdrawal_limit
        {
            0 => self.calculate_current_withdrawal_limit_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "calculate_current_withdrawal_limit".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
