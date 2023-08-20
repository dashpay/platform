mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Removes a specified amount from the system credits.
    ///
    /// This function is used when an identity withdraws some of their balance.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of system credits to be removed.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used for removing the system credits.
    /// * `drive_version` - A `DriveVersion` object specifying the version of the Drive.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns Ok(()). If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    pub fn remove_from_system_credits(
        &self,
        amount: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .balances
            .remove_from_system_credits
        {
            0 => self.remove_from_system_credits_v0(amount, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_from_system_credits".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
