use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adds to the total platform system credits when:
    /// - we create an identity
    /// - we top up an identity
    /// - through the block reward
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of system credits to be added.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used for adding to the system credits.
    /// * `platform_version` - A `PlatformVersion` object specifying the version of Platform.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns `Ok(())`. If an error occurs during the operation, returns an `Error`.
    #[inline(always)]
    pub(super) fn add_to_system_credits_v0(
        &self,
        amount: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut drive_operations = vec![];
        let batch_operations = self.add_to_system_credits_operations(
            amount,
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
        )
    }
}
