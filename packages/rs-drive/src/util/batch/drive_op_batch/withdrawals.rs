use std::collections::HashMap;

use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path_vec, get_withdrawal_transactions_broadcasted_path_vec,
    get_withdrawal_transactions_queue_path_vec, get_withdrawal_transactions_sum_tree_path_vec,
    WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY,
};
use crate::util::grove_operations::{
    BatchDeleteApplyType, BatchInsertApplyType, BatchMoveApplyType,
};
use crate::util::object_size_info::PathKeyElementInfo;
use crate::{drive::Drive, error::Error, fees::op::LowLevelDriveOperation};
use dpp::block::block_info::BlockInfo;

use super::DriveLowLevelOperationConverter;
use crate::query::Query;
use dpp::fee::{Credits, SignedCredits};
use dpp::prelude::TimestampMillis;
use dpp::withdrawal::{WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes};
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, MaybeTree, TransactionArg};
use grovedb::{Element, PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;

/// Operations for Withdrawals
#[derive(Clone, Debug)]
pub enum WithdrawalOperationType {
    /// Update index counter
    UpdateIndexCounter {
        /// index counter value
        index: WithdrawalTransactionIndex,
    },
    /// Insert Core Transaction into queue
    InsertTransactions {
        /// transaction id bytes
        withdrawal_transactions: Vec<WithdrawalTransactionIndexAndBytes>,
    },
    /// Deletes the withdrawal transactions from the main queue and adds them to the broadcasted queue
    MoveWithdrawalTransactionsToBroadcasted {
        /// A vector of the indexes to be moved
        indexes: Vec<WithdrawalTransactionIndex>,
    },
    /// Deletes the withdrawal transactions from the main queue and adds them to the broadcasted queue
    MoveBroadcastedWithdrawalTransactionsBackToQueueForResigning {
        /// A vector of the indexes to be moved
        indexes: Vec<WithdrawalTransactionIndex>,
    },
    /// Deletes the withdrawal transactions from the broadcasted queue
    DeleteCompletedBroadcastedWithdrawalTransactions {
        /// A vector of the indexes to be deleted
        indexes: Vec<WithdrawalTransactionIndex>,
    },
    /// Reserve an amount in the system for withdrawals, the reservation will expire at the date given
    ReserveWithdrawalAmount {
        /// amount to reserve
        amount: Credits,
        /// expiration date
        expiration_after: TimestampMillis,
    },
}

impl DriveLowLevelOperationConverter for WithdrawalOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        _estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            WithdrawalOperationType::UpdateIndexCounter { index } => {
                let mut drive_operations = vec![];

                let path = get_withdrawal_root_path_vec();

                drive.batch_insert(
                    PathKeyElementInfo::PathKeyRefElement::<'_, 1>((
                        path,
                        &WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY,
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

                for (index, bytes) in withdrawal_transactions {
                    drive.batch_insert(
                        PathKeyElementInfo::PathKeyElement::<'_, 0>((
                            path.clone(),
                            index.to_be_bytes().to_vec(),
                            Element::Item(bytes, None),
                        )),
                        &mut drive_operations,
                        &platform_version.drive,
                    )?;
                }

                Ok(drive_operations)
            }
            WithdrawalOperationType::ReserveWithdrawalAmount {
                amount,
                expiration_after,
            } => {
                let mut drive_operations = vec![];

                let expiration_date = block_info.time_ms + expiration_after;

                let sum_path = get_withdrawal_transactions_sum_tree_path_vec();

                drive.batch_insert_sum_item_or_add_to_if_already_exists(
                    PathKeyElementInfo::PathKeyElement::<'_, 0>((
                        sum_path.clone(),
                        expiration_date.to_be_bytes().to_vec(),
                        Element::SumItem(amount as SignedCredits, None),
                    )),
                    BatchInsertApplyType::StatefulBatchInsert,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::MoveWithdrawalTransactionsToBroadcasted { indexes } => {
                let mut drive_operations = vec![];

                if indexes.is_empty() {
                    return Ok(drive_operations);
                }

                let original_path = get_withdrawal_transactions_queue_path_vec();
                let new_path = get_withdrawal_transactions_broadcasted_path_vec();

                let mut query = Query::new();

                let len = indexes.len();

                query.insert_keys(
                    indexes
                        .into_iter()
                        .map(|index| index.to_be_bytes().to_vec())
                        .collect(),
                );

                let path_query = PathQuery::new(
                    original_path,
                    SizedQuery::new(query, Some(len as u16), None),
                );

                drive.batch_move_items_in_path_query(
                    &path_query,
                    new_path,
                    true,
                    // we know that we are not deleting a subtree
                    BatchMoveApplyType::StatefulBatchMove {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    None,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::MoveBroadcastedWithdrawalTransactionsBackToQueueForResigning { indexes } => {
                let mut drive_operations = vec![];

                if indexes.is_empty() {
                    return Ok(drive_operations);
                }

                let original_path = get_withdrawal_transactions_broadcasted_path_vec();
                let new_path = get_withdrawal_transactions_queue_path_vec();

                let mut query = Query::new();

                let len = indexes.len();

                query.insert_keys(
                    indexes
                        .into_iter()
                        .map(|index| index.to_be_bytes().to_vec())
                        .collect(),
                );

                let path_query = PathQuery::new(
                    original_path,
                    SizedQuery::new(query, Some(len as u16), None),
                );

                drive.batch_move_items_in_path_query(
                    &path_query,
                    new_path,
                    true,
                    // we know that we are not deleting a subtree
                    BatchMoveApplyType::StatefulBatchMove {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    None,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::DeleteCompletedBroadcastedWithdrawalTransactions { indexes } => {
                let mut drive_operations = vec![];

                if indexes.is_empty() {
                    return Ok(drive_operations);
                }

                let path = get_withdrawal_transactions_broadcasted_path_vec();

                let mut query = Query::new();

                let len = indexes.len();

                query.insert_keys(
                    indexes
                        .into_iter()
                        .map(|index| index.to_be_bytes().to_vec())
                        .collect(),
                );

                let path_query = PathQuery::new(
                    path,
                    SizedQuery::new(query, Some(len as u16), None),
                );

                drive.batch_delete_items_in_path_query(
                    &path_query,
                    true,
                    // we know that we are not deleting a subtree
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
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
