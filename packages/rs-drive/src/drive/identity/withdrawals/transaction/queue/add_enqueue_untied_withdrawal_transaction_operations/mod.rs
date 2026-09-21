use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::DriveOperation;
use dpp::fee::Credits;
use dpp::withdrawal::{WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes};
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

impl Drive {
    /// Add insert operations for withdrawal transactions to the batch, and count their
    /// amounts against the withdrawal limit: as one reservation of the block's total that
    /// expires after a day up to protocol version 13, as one in-flight entry per transaction
    /// index from protocol version 14.
    pub fn add_enqueue_untied_withdrawal_transaction_operations(
        &self,
        withdrawal_transactions: Vec<WithdrawalTransactionIndexAndBytes>,
        amounts: Vec<(WithdrawalTransactionIndex, Credits)>,
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
                let total_sum = amounts
                    .iter()
                    .try_fold(0u64, |total, (_, amount)| total.checked_add(*amount))
                    .ok_or(Error::Protocol(Box::new(dpp::ProtocolError::Overflow(
                        "overflow in the total withdrawal amount",
                    ))))?;
                self.add_enqueue_untied_withdrawal_transaction_operations_v0(
                    withdrawal_transactions,
                    total_sum,
                    drive_operation_types,
                );
                Ok(())
            }
            1 => {
                self.add_enqueue_untied_withdrawal_transaction_operations_v1(
                    withdrawal_transactions,
                    amounts,
                    drive_operation_types,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_enqueue_untied_withdrawal_transaction_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
