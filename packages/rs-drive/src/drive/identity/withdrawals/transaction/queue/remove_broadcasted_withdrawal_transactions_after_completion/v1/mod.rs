use crate::drive::Drive;
use crate::util::batch::drive_op_batch::WithdrawalOperationType;
use crate::util::batch::DriveOperation;
use dpp::withdrawal::WithdrawalTransactionIndex;

impl Drive {
    /// Drops the broadcast transactions and releases the withdrawals from the in-flight sum
    /// tree, so they no longer count against the withdrawal limit.
    pub(super) fn remove_broadcasted_withdrawal_transactions_after_completion_operations_v1(
        &self,
        indexes: Vec<WithdrawalTransactionIndex>,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::DeleteCompletedBroadcastedWithdrawalTransactions {
                indexes: indexes.clone(),
            },
        ));
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::ReleaseInFlightWithdrawals { indexes },
        ));
    }
}
