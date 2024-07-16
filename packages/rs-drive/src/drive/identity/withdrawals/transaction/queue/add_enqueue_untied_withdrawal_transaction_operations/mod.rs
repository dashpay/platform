use crate::drive::identity::withdrawals::WithdrawalTransactionIndexAndBytes;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::DriveOperation;
use platform_version::version::PlatformVersion;

mod v0;

impl Drive {
    /// Add insert operations for withdrawal transactions to the batch
    pub fn add_enqueue_untied_withdrawal_transaction_operations(
        &self,
        withdrawal_transactions: Vec<WithdrawalTransactionIndexAndBytes>,
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
            .add_enqueue_untied_withdrawal_transaction_operations
        {
            0 => {
                self.add_enqueue_untied_withdrawal_transaction_operations_v0(
                    withdrawal_transactions,
                    drive_operation_types,
                );

                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_enqueue_untied_withdrawal_transaction_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
