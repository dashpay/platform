use crate::drive::identity::withdrawals::WithdrawalTransactionIndexAndBytes;
use crate::drive::Drive;
use crate::util::batch::drive_op_batch::WithdrawalOperationType;
use crate::util::batch::DriveOperation;

impl Drive {
    pub(super) fn add_enqueue_untied_withdrawal_transaction_operations_v0(
        &self,
        withdrawal_transactions: Vec<WithdrawalTransactionIndexAndBytes>,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        if !withdrawal_transactions.is_empty() {
            drive_operation_types.push(DriveOperation::WithdrawalOperation(
                WithdrawalOperationType::InsertTransactions {
                    withdrawal_transactions,
                },
            ));
        }
    }
}
