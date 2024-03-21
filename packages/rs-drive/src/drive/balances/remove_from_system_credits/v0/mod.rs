use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// We remove from system credits when:
    /// - an identity withdraws some of their balance
    #[inline(always)]
    pub(super) fn remove_from_system_credits_v0(
        &self,
        amount: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut drive_operations = vec![];
        let batch_operations = self.remove_from_system_credits_operations(
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
