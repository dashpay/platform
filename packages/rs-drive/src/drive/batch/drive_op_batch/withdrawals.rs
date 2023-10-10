use std::collections::HashMap;

use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::Element;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};

use crate::drive::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use crate::drive::grove_operations::BatchDeleteApplyType;
use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path_vec, get_withdrawal_transactions_expired_ids_path,
    get_withdrawal_transactions_expired_ids_path_vec, get_withdrawal_transactions_queue_path,
    get_withdrawal_transactions_queue_path_vec, WITHDRAWAL_TRANSACTIONS_COUNTER_ID,
};
use crate::drive::identity::withdrawals::WithdrawalTransactionIdAndBytes;
use crate::drive::object_size_info::PathKeyElementInfo;
use crate::{drive::Drive, error::Error, fee::op::LowLevelDriveOperation};

use super::DriveLowLevelOperationConverter;

/// Operations for Withdrawals
#[derive(Clone, Debug)]
pub enum WithdrawalOperationType<'a> {
    /// Inserts expired index into it's tree
    InsertExpiredIndex {
        /// index value
        index: u64,
    },
    /// Removes expired index from the tree
    DeleteExpiredIndex {
        /// index value
        key: Vec<u8>,
    },
    /// Update index counter
    UpdateIndexCounter {
        /// index counter value
        index: u64,
    },
    /// Insert Core Transaction into queue
    InsertTransactions {
        /// transaction id bytes
        withdrawal_transactions: &'a [WithdrawalTransactionIdAndBytes],
    },
    /// Delete withdrawal
    DeleteWithdrawalTransaction {
        /// withdrawal transaction tuple with id and bytes
        id: Vec<u8>,
    },
}

impl DriveLowLevelOperationConverter for WithdrawalOperationType<'_> {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        _estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            WithdrawalOperationType::InsertExpiredIndex { index } => {
                let mut drive_operations = vec![];

                let index_bytes = index.to_be_bytes();

                let path = get_withdrawal_transactions_expired_ids_path_vec();

                drive.batch_insert(
                    PathKeyElementInfo::PathKeyElement::<'_, 1>((
                        path,
                        index_bytes.to_vec(),
                        Element::Item(vec![], None),
                    )),
                    &mut drive_operations,
                    &platform_version.drive,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::DeleteExpiredIndex { key } => {
                let mut drive_operations = vec![];

                let path: [&[u8]; 2] = get_withdrawal_transactions_expired_ids_path();

                drive.batch_delete(
                    (&path).into(),
                    &key,
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some((false, false)),
                    },
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::UpdateIndexCounter { index } => {
                let mut drive_operations = vec![];

                let path = get_withdrawal_root_path_vec();

                drive.batch_insert(
                    PathKeyElementInfo::PathKeyRefElement::<'_, 1>((
                        path,
                        &WITHDRAWAL_TRANSACTIONS_COUNTER_ID,
                        Element::Item(index.to_be_bytes().to_vec(), None),
                    )),
                    &mut drive_operations,
                    &platform_version.drive,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::InsertTransactions {
                withdrawal_transactions,
            } => {
                let mut drive_operations = vec![];

                let path = get_withdrawal_transactions_queue_path_vec();

                for (id, bytes) in withdrawal_transactions {
                    drive.batch_insert(
                        PathKeyElementInfo::PathKeyElement::<'_, 1>((
                            path.clone(),
                            id.clone(),
                            Element::Item(bytes.clone(), None),
                        )),
                        &mut drive_operations,
                        &platform_version.drive,
                    )?;
                }

                Ok(drive_operations)
            }
            WithdrawalOperationType::DeleteWithdrawalTransaction { id } => {
                let mut drive_operations = vec![];

                let path = get_withdrawal_transactions_queue_path();

                drive.batch_delete(
                    (&path).into(),
                    &id,
                    // we know that we are not deleting a subtree
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some((false, false)),
                    },
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;

                Ok(drive_operations)
            }
        }
    }
}
