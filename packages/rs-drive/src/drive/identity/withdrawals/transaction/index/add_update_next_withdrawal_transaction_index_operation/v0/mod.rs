use crate::drive::batch::drive_op_batch::WithdrawalOperationType;
use crate::drive::batch::DriveOperation;
use crate::drive::identity::withdrawals::WithdrawalTransactionIndex;
use crate::drive::Drive;

impl Drive {
    pub(super) fn add_update_next_withdrawal_transaction_index_operation_v0(
        &self,
        index: WithdrawalTransactionIndex,
        drive_operation_types: &mut Vec<DriveOperation>,
    ) {
        drive_operation_types.push(DriveOperation::WithdrawalOperation(
            WithdrawalOperationType::UpdateIndexCounter { index },
        ));
    }
}
