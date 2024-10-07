use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::DriveOperation;
use dpp::withdrawal::WithdrawalTransactionIndex;
use platform_version::version::PlatformVersion;

mod v0;

impl Drive {
    /// Moves broadcasted withdrawal transactions back to the queue
    pub fn move_broadcasted_withdrawal_transactions_back_to_queue_operations(
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
            .move_broadcasted_withdrawal_transactions_back_to_queue_operations
        {
            0 => {
                self.move_broadcasted_withdrawal_transactions_back_to_queue_operations_v0(
                    indexes,
                    drive_operation_types,
                );

                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "move_broadcasted_withdrawal_transactions_back_to_queue_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
