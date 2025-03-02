use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Empties the prefunded specialized balance
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
    #[inline(always)]
    pub(super) fn empty_prefunded_specialized_balance_v0(
        &self,
        specialized_balance_id: Identifier,
        error_if_does_not_exist: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        let mut drive_operations = vec![];
        let (credits, batch_operations) = self.empty_prefunded_specialized_balance_operations(
            specialized_balance_id,
            error_if_does_not_exist,
            &mut None,
            transaction,
            platform_version,
        )?;
        let grove_db_operations =
            LowLevelDriveOperation::grovedb_operations_batch(&batch_operations);
        self.grove_apply_batch_with_add_costs(
            grove_db_operations,
            false,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok(credits)
    }
}
