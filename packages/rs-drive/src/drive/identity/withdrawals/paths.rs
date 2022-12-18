use grovedb::Element;

use crate::drive::{batch::GroveDbOpBatch, RootTree};

/// constant id for transaction counter
pub const WITHDRAWAL_TRANSACTIONS_COUNTER_ID: [u8; 1] = [0];
/// constant id for subtree containing transactions queue
pub const WITHDRAWAL_TRANSACTIONS_QUEUE_ID: [u8; 1] = [1];
/// constant id for subtree containing expired transaction ids
pub const WITHDRAWAL_TRANSACTIONS_EXPIRED_IDS: [u8; 1] = [2];

/// Simple type alias for withdrawal transaction with it's id
pub type WithdrawalTransaction = (Vec<u8>, Vec<u8>);

/// Add operations for creating initial withdrawal state structure
pub fn add_initial_withdrawal_state_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_tree(vec![], vec![RootTree::WithdrawalTransactions as u8]);

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

/// Helper function to get queue path as Vec
pub fn get_withdrawal_transactions_queue_path() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::WithdrawalTransactions as u8],
        WITHDRAWAL_TRANSACTIONS_QUEUE_ID.to_vec(),
    ]
}

/// Helper function to get queue path as [u8]
pub fn get_withdrawal_transactions_queue_path_as_u8() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions),
        &WITHDRAWAL_TRANSACTIONS_QUEUE_ID,
    ]
}

/// Helper function to get expired ids path as Vec
pub fn get_withdrawal_transactions_expired_ids_path() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::WithdrawalTransactions as u8],
        WITHDRAWAL_TRANSACTIONS_EXPIRED_IDS.to_vec(),
    ]
}

/// Helper function to get expired ids path as [u8]
pub fn get_withdrawal_transactions_expired_ids_path_as_u8() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions),
        &WITHDRAWAL_TRANSACTIONS_EXPIRED_IDS,
    ]
}
