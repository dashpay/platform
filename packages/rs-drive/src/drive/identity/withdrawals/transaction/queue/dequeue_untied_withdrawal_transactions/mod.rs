mod v0;

use crate::drive::identity::withdrawals::WithdrawalTransactionIndexAndBytes;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::DriveOperation;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Get specified amount of withdrawal transactions from the DB
    pub fn dequeue_untied_withdrawal_transactions(
        &self,
        limit: u16,
        transaction: TransactionArg,
        drive_operation_types: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<WithdrawalTransactionIndexAndBytes>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .transaction
            .queue
            .dequeue_untied_withdrawal_transactions
        {
            0 => self.dequeue_untied_withdrawal_transactions_v0(
                limit,
                transaction,
                drive_operation_types,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "dequeue_untied_withdrawal_transactions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
