mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Empties from a prefunded specialized balance the entire left over balance
    ///
    /// # Arguments
    ///
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
    pub fn empty_prefunded_specialized_balance(
        &self,
        specialized_balance_id: Identifier,
        error_if_does_not_exist: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        match platform_version
            .drive
            .methods
            .prefunded_specialized_balances
            .empty_prefunded_specialized_balance
        {
            0 => self.empty_prefunded_specialized_balance_v0(
                specialized_balance_id,
                error_if_does_not_exist,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "empty_prefunded_specialized_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
