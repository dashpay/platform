use crate::drive::Drive;
use crate::util::batch::drive_op_batch::WithdrawalOperationType;
use crate::util::batch::DriveOperation;
use dpp::fee::Credits;
use dpp::withdrawal::{WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes};

impl Drive {
    /// Queues the transactions and records each one as in flight under its index, where it
    /// counts against the withdrawal limit until Core mines it or it fails for good.
    pub(super) fn add_enqueue_untied_withdrawal_transaction_operations_v1(
        &self,
        withdrawal_transactions: Vec<WithdrawalTransactionIndexAndBytes>,
        amounts: Vec<(WithdrawalTransactionIndex, Credits)>,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        if withdrawal_transactions.is_empty() {
            return;
        }
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::InsertTransactions {
                withdrawal_transactions,
            },
        ));
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::ReserveInFlightWithdrawals { amounts },
        ));
    }
}
