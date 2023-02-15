use std::collections::HashMap;

use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};

use crate::{
    drive::{block_info::BlockInfo, identity::withdrawals::paths::WithdrawalTransaction, Drive},
    error::Error,
    fee::op::DriveOperation,
};

use super::DriveOperationConverter;

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
        withdrawal_transactions: &'a [WithdrawalTransaction],
    },
    /// Delete withdrawal
    DeleteWithdrawalTransaction {
        /// withdrawal transaction tuple with id and bytes
        id: Vec<u8>,
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
                drive.insert_withdrawal_expired_index(index)
            }
            WithdrawalOperationType::DeleteExpiredIndex { key } => {
                drive.delete_withdrawal_expired_index(key, transaction)
            }
            WithdrawalOperationType::UpdateIndexCounter { index } => {
                drive.update_transaction_index_counter(index)
            }
            WithdrawalOperationType::InsertTransactions {
                withdrawal_transactions,
            } => drive.insert_withdrawal_transactions(withdrawal_transactions),
            WithdrawalOperationType::DeleteWithdrawalTransaction { id } => {
                drive.delete_withdrawal_transaction(id.as_slice(), transaction)
            }
        }
    }
}
