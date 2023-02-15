use crate::drive::grove_operations::BatchDeleteApplyType;
use crate::drive::identity::withdrawals::paths::get_withdrawal_transactions_queue_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use grovedb::TransactionArg;

impl Drive {
    pub(crate) fn delete_withdrawal_transaction(
        &self,
        id: &[u8],
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];

        let path = get_withdrawal_transactions_queue_path();

        self.batch_delete(
            path,
            id,
            // we know that we are not deleting a subtree
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some((false, false)),
            },
            transaction,
            &mut drive_operations,
        )?;

        Ok(drive_operations)
    }
}
