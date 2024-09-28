use crate::drive::Drive;
use crate::util::batch::drive_op_batch::WithdrawalOperationType;
use crate::util::batch::DriveOperation;
use dpp::fee::Credits;
use dpp::prelude::TimestampMillis;
use dpp::withdrawal::WithdrawalTransactionIndexAndBytes;

pub const DAY_IN_MS: TimestampMillis = 86_400_000;

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
                    expiration_after: DAY_IN_MS,
                },
            ));
        }
    }
}
