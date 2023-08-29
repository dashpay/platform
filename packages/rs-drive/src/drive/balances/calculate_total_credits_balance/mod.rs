

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::balances::total_credits_balance::TotalCreditsBalance;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

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
    /// * `Result<TotalCreditsBalance, Error>` - If successful, returns a `TotalCreditsBalance` object representing the total credits balance.
    ///   If an error occurs during the calculation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    pub fn calculate_total_credits_balance(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<TotalCreditsBalance, Error> {
        match drive_version
            .methods
            .balances
            .calculate_total_credits_balance
        {
            0 => self.calculate_total_credits_balance_v0(transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "calculate_total_credits_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
