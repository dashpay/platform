use crate::drive::grove_operations::BatchDeleteApplyType;
use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_transactions_expired_ids_path, get_withdrawal_transactions_expired_ids_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use grovedb::{Element, TransactionArg};

impl Drive {
    pub(crate) fn insert_withdrawal_expired_index(
        &self,
        index: u64,
    ) -> Result<(Vec<DriveOperation>), Error> {
        let mut drive_operations = vec![];

        let index_bytes = index.to_be_bytes();

        let path = get_withdrawal_transactions_expired_ids_path_vec();

        self.batch_insert(
            crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement::<'_, 1>((
                path,
                index_bytes.to_vec(),
                Element::Item(vec![], None),
            )),
            &mut drive_operations,
        )?;

        Ok(drive_operations)
    }

    pub(crate) fn delete_withdrawal_expired_index(
        &self,
        key: &[u8],
        transaction: TransactionArg,
    ) -> Result<(Vec<DriveOperation>), Error> {
        let mut drive_operations = vec![];

        let path: [&[u8]; 2] = get_withdrawal_transactions_expired_ids_path();

        self.batch_delete(
            path,
            key,
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some((false, false)),
            },
            transaction,
            &mut drive_operations,
        )?;

        Ok(drive_operations)
    }
}
