mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::DriveOperation;
use dpp::withdrawal::WithdrawalTransactionIndex;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Get specified amount of withdrawal transactions from the DB
    pub fn remove_broadcasted_withdrawal_transactions_after_completion_operations(
        &self,
        indexes: Vec<WithdrawalTransactionIndex>,
        drive_operation_types: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .transaction
            .queue
            .remove_broadcasted_withdrawal_transactions_after_completion_operations
        {
            0 => {
                self.remove_broadcasted_withdrawal_transactions_after_completion_operations_v0(
                    indexes,
                    drive_operation_types,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_broadcasted_withdrawal_transactions_after_completion_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
