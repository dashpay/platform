mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adds a new prefunded specialized balance
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of credits to be added to the prefunded balance.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used for adding to the system credits.
    /// * `platform_version` - A `PlatformVersion` object specifying the version of Platform.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns `Ok(())`. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of Platform is unknown.
    pub fn add_prefunded_specialized_balance(
        &self,
        specialized_balance_id: Identifier,
        amount: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .prefunded_specialized_balances
            .add_prefunded_specialized_balance
        {
            0 => self.add_prefunded_specialized_balance_v0(
                specialized_balance_id,
                amount,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_prefunded_specialized_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
