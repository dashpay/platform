use crate::drive::Drive;
use crate::util::batch::drive_op_batch::WithdrawalOperationType;
use crate::util::batch::DriveOperation;
use dpp::withdrawal::WithdrawalTransactionIndex;

impl Drive {
    pub(super) fn move_broadcasted_withdrawal_transactions_back_to_queue_operations_v0(
        &self,
        indexes: Vec<WithdrawalTransactionIndex>,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        if !indexes.is_empty() {
            drive_operation_types.push(DriveOperation::WithdrawalOperation(
                WithdrawalOperationType::MoveBroadcastedWithdrawalTransactionsBackToQueueForResigning {
                    indexes,
                },
            ));
        }
    }
}
