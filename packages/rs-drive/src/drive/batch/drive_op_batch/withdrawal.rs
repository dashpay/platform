use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use crate::drive::batch::drive_op_batch::DriveOperationConverter;
use crate::drive::block_info::BlockInfo;
use crate::drive::Drive;
use crate::drive::grove_operations::BatchDeleteApplyType;
use crate::drive::identity::withdrawals::paths::{get_withdrawal_root_path, get_withdrawal_transactions_expired_ids_path, get_withdrawal_transactions_expired_ids_path_as_u8, get_withdrawal_transactions_queue_path, WITHDRAWAL_TRANSACTIONS_COUNTER_ID, WithdrawalTransaction};
use crate::error::Error;
use crate::fee::op::DriveOperation;

/// Operations for Withdrawals
pub enum WithdrawalOperationType<'a> {
    /// Inserts expired index into it's tree
    InsertExpiredIndex {
        /// index value
        index: u64,
    },
    /// Removes expired index from the tree
    DeleteExpiredIndex {
        /// index value
        key: &'a [u8],
    },
    /// Update index counter
    UpdateIndexCounter {
        /// index counter value
        index: u64,
    },
    /// Insert Core Transaction into queue
    InsertTransactions {
        /// transaction id bytes
        transactions: &'a [WithdrawalTransaction],
    },
}

impl DriveOperationConverter for WithdrawalOperationType<'_> {
    fn to_drive_operations(
        self,
        drive: &Drive,
        _estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        match self {
            WithdrawalOperationType::InsertExpiredIndex { index } => {
                let mut drive_operations = vec![];

                let index_bytes = index.to_be_bytes().to_vec();

                let path = get_withdrawal_transactions_expired_ids_path();

                drive.batch_insert(
                    crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement::<'_, 1>((
                        path,
                        index_bytes,
                        Element::Item(vec![], None),
                    )),
                    &mut drive_operations,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::DeleteExpiredIndex { key } => {
                let mut drive_operations = vec![];

                let path: [&[u8]; 2] = get_withdrawal_transactions_expired_ids_path_as_u8();

                drive.batch_delete(
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
            WithdrawalOperationType::UpdateIndexCounter { index } => {
                let mut drive_operations = vec![];

                let path = get_withdrawal_root_path();

                drive.batch_insert(
                    crate::drive::object_size_info::PathKeyElementInfo::PathKeyRefElement::<'_, 1>((
                        path,
                        &WITHDRAWAL_TRANSACTIONS_COUNTER_ID,
                        Element::Item(index.to_be_bytes().to_vec(), None),
                    )),
                    &mut drive_operations,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::InsertTransactions { transactions } => {
                let mut drive_operations = vec![];

                let path = get_withdrawal_transactions_queue_path();

                for (id, bytes) in transactions {
                    drive.batch_insert(
                        crate::drive::object_size_info::PathKeyElementInfo::PathKeyRefElement::<'_, 1>(
                            (path.clone(), id, Element::Item(bytes.clone(), None)),
                        ),
                        &mut drive_operations,
                    )?;
                }

                Ok(drive_operations)
            }
        }
    }
}
