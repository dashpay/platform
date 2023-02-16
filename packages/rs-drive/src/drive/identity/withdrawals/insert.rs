use crate::drive::identity::withdrawals::paths::get_withdrawal_transactions_queue_path_vec;
use crate::drive::identity::withdrawals::WithdrawalTransactionIdAndBytes;
use crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use grovedb::Element;

impl Drive {
    pub(crate) fn insert_withdrawal_transactions(
        &self,
        transactions: &[WithdrawalTransactionIdAndBytes],
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];

        let path = get_withdrawal_transactions_queue_path_vec();

        for (id, bytes) in transactions {
            self.batch_insert(
                PathKeyElement::<'_, 1>((
                    path.clone(),
                    id.clone(),
                    Element::Item(bytes.clone(), None),
                )),
                &mut drive_operations,
            )?;
        }

        Ok(drive_operations)
    }
}
