use crate::drive::identity::withdrawals::DAY_AND_A_HOUR_IN_MS;
use crate::drive::Drive;
use crate::util::batch::drive_op_batch::WithdrawalOperationType;
use crate::util::batch::DriveOperation;
use dpp::fee::Credits;
use dpp::withdrawal::WithdrawalTransactionIndexAndBytes;

impl Drive {
    pub(super) fn add_enqueue_untied_withdrawal_transaction_operations_v0(
        &self,
        withdrawal_transactions: Vec<WithdrawalTransactionIndexAndBytes>,
        total_sum: Credits,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        if !withdrawal_transactions.is_empty() {
            drive_operation_types.push(DriveOperation::WithdrawalOperation(
                WithdrawalOperationType::InsertTransactions {
                    withdrawal_transactions,
                },
            ));
            drive_operation_types.push(DriveOperation::WithdrawalOperation(
                WithdrawalOperationType::ReserveWithdrawalAmount {
                    amount: total_sum,
                    expiration_after: DAY_AND_A_HOUR_IN_MS,
                },
            ));
        }
    }
}
