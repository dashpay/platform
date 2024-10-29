use crate::drive::Drive;
use crate::util::batch::drive_op_batch::WithdrawalOperationType;
use crate::util::batch::DriveOperation;
use dpp::withdrawal::WithdrawalTransactionIndex;
impl Drive {
    pub(super) fn remove_broadcasted_withdrawal_transactions_after_completion_operations_v0(
        &self,
        indexes: Vec<WithdrawalTransactionIndex>,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::DeleteCompletedBroadcastedWithdrawalTransactions { indexes },
        ));
    }
}
