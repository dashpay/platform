use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Deducts from a prefunded specialized balance (v1).
    ///
    /// Functionally identical to `deduct_from_prefunded_specialized_balance_v0`:
    /// version selection for the actual deduction logic happens inside the
    /// `deduct_from_prefunded_specialized_balance_operations` dispatcher, which
    /// routes to `_operations_v0` / `_operations_v1` based on the platform
    /// version. This top-level wrapper just collects those operations and
    /// applies the resulting grovedb batch.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of credits to be removed from the prefunded balance.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used.
    /// * `platform_version` - A `PlatformVersion` object specifying the version of Platform.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns `Ok(())`. Otherwise, returns an `Error`.
    #[inline(always)]
    pub(super) fn deduct_from_prefunded_specialized_balance_v1(
        &self,
        specialized_balance_id: Identifier,
        amount: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut drive_operations = vec![];
        let batch_operations = self.deduct_from_prefunded_specialized_balance_operations(
            specialized_balance_id,
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
