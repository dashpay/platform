mod v0;

use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::DriveOperation;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Add next transaction index increment operation to the batch
    pub fn add_update_next_withdrawal_transaction_index_operation(
        &self,
        index: WithdrawalTransactionIndex,
        drive_operation_types: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .transaction
            .index
            .add_update_next_withdrawal_transaction_index_operation
        {
            0 => {
                self.add_update_next_withdrawal_transaction_index_operation_v0(
                    index,
                    drive_operation_types,
                );

                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_update_next_withdrawal_transaction_index_operation".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
