use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

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
    #[inline(always)]
    pub(super) fn add_prefunded_specialized_balance_v0(
        &self,
        specialized_balance_id: Identifier,
        amount: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut drive_operations = vec![];
        let batch_operations = self.add_prefunded_specialized_balance_operations(
            specialized_balance_id,
            amount,
            &mut None,
            transaction,
            platform_version,
        )?;
        let grove_db_operations =
            LowLevelDriveOperation::grovedb_operations_batch_consume(batch_operations);
        self.grove_apply_batch_with_add_costs(
            grove_db_operations,
            false,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )
    }
}
