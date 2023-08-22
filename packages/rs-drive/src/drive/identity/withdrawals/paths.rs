use grovedb::Element;

use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::{batch::GroveDbOpBatch, RootTree};

/// constant id for transaction counter
pub const WITHDRAWAL_TRANSACTIONS_COUNTER_ID: [u8; 1] = [0];
/// constant id for subtree containing transactions queue
pub const WITHDRAWAL_TRANSACTIONS_QUEUE_ID: [u8; 1] = [1];
/// constant id for subtree containing expired transaction ids
pub const WITHDRAWAL_TRANSACTIONS_EXPIRED_IDS: [u8; 1] = [2];

/// Add operations for creating initial withdrawal state structure
pub fn add_initial_withdrawal_state_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert(
        vec![vec![RootTree::WithdrawalTransactions as u8]],
        WITHDRAWAL_TRANSACTIONS_COUNTER_ID.to_vec(),
        Element::Item(0u64.to_be_bytes().to_vec(), None),
    );

    batch.add_insert_empty_tree(
        vec![vec![RootTree::WithdrawalTransactions as u8]],
        WITHDRAWAL_TRANSACTIONS_QUEUE_ID.to_vec(),
    );

    batch.add_insert_empty_tree(
        vec![vec![RootTree::WithdrawalTransactions as u8]],
        WITHDRAWAL_TRANSACTIONS_EXPIRED_IDS.to_vec(),
    );
}

/// Helper function to get root path
pub fn get_withdrawal_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::WithdrawalTransactions as u8]]
}

/// Helper function to get root path as u8
pub fn get_withdrawal_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions)]
}

/// Helper function to get queue path as Vec
pub fn get_withdrawal_transactions_queue_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::WithdrawalTransactions as u8],
        WITHDRAWAL_TRANSACTIONS_QUEUE_ID.to_vec(),
    ]
}

/// Helper function to get queue path as [u8]
pub fn get_withdrawal_transactions_queue_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions),
        &WITHDRAWAL_TRANSACTIONS_QUEUE_ID,
    ]
}

/// Helper function to get expired ids path as Vec
pub fn get_withdrawal_transactions_expired_ids_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::WithdrawalTransactions as u8],
        WITHDRAWAL_TRANSACTIONS_EXPIRED_IDS.to_vec(),
    ]
}

/// Helper function to get expired ids path as [u8]
pub fn get_withdrawal_transactions_expired_ids_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions),
        &WITHDRAWAL_TRANSACTIONS_EXPIRED_IDS,
    ]
}
