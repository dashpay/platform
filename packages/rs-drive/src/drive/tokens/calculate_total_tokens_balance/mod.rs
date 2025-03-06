mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::balances::total_tokens_balance::TotalTokensBalance;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Calculates the total credits balance.
    ///
    /// This function verifies that the sum tree identity credits + pool credits + refunds are equal to the total credits in the system.
    ///
    /// # Arguments
    ///
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used for calculating the total credits balance.
    /// * `drive_version` - A `DriveVersion` object specifying the version of the Drive.
    ///
    /// # Returns
    ///
    /// * `Result<TotalTokensBalance, Error>` - If successful, returns a `TotalTokensBalance` object representing the total tokens balance.
    ///   If an error occurs during the calculation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    pub fn calculate_total_tokens_balance(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<TotalTokensBalance, Error> {
        match platform_version
            .drive
            .methods
            .token
            .calculate_total_tokens_balance
        {
            0 => self.calculate_total_tokens_balance_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "calculate_total_tokens_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
