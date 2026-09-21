use crate::drive::{Drive, RootTree};
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use grovedb::Element;
use platform_version::version::PlatformVersion;

/// constant key for transaction counter
pub const WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY: [u8; 1] = [0];
/// constant id for subtree containing transactions queue
pub const WITHDRAWAL_TRANSACTIONS_QUEUE_KEY: [u8; 1] = [1];
/// constant id for the sum tree of withdrawal amounts. Up to protocol version 13 an entry is a
/// pooling block's total keyed by the expiration of its reservation (block time in
/// milliseconds, big-endian); from protocol version 14 an entry is one pooled withdrawal keyed
/// by its transaction index, held while the withdrawal is in flight and removed once Core has
/// mined it or it failed for good, so the tree's sum is the amount in flight.
pub const WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY: [u8; 1] = [2];
/// constant id for subtree containing the untied withdrawal transactions after they were broadcasted
pub const WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY: [u8; 1] = [3];

impl Drive {
    /// Add operations for creating initial withdrawal state structure
    pub fn add_initial_withdrawal_state_structure_operations(
        batch: &mut GroveDbOpBatch,
        platform_version: &PlatformVersion,
    ) {
        batch.add_insert(
            vec![vec![RootTree::WithdrawalTransactions as u8]],
            WITHDRAWAL_TRANSACTIONS_NEXT_INDEX_KEY.to_vec(),
            Element::Item(0u64.to_be_bytes().to_vec(), None),
        );

        batch.add_insert_empty_tree(
            vec![vec![RootTree::WithdrawalTransactions as u8]],
            WITHDRAWAL_TRANSACTIONS_QUEUE_KEY.to_vec(),
        );

        if platform_version.protocol_version >= 4 {
            batch.add_insert_empty_sum_tree(
                vec![vec![RootTree::WithdrawalTransactions as u8]],
                WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY.to_vec(),
            );
            batch.add_insert_empty_sum_tree(
                vec![vec![RootTree::WithdrawalTransactions as u8]],
                WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY.to_vec(),
            );
        }
    }
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
        WITHDRAWAL_TRANSACTIONS_QUEUE_KEY.to_vec(),
    ]
}

/// Helper function to get queue path as [u8]
pub fn get_withdrawal_transactions_queue_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions),
        &WITHDRAWAL_TRANSACTIONS_QUEUE_KEY,
    ]
}

/// Helper function to get the withdrawal transactions sum tree path as Vec
pub fn get_withdrawal_transactions_sum_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::WithdrawalTransactions as u8],
        WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY.to_vec(),
    ]
}

/// Helper function to get the withdrawal transactions sum tree path as [u8]
pub fn get_withdrawal_transactions_sum_tree_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions),
        &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
    ]
}

/// Helper function to get the withdrawal transactions broadcasted path as Vec
pub fn get_withdrawal_transactions_broadcasted_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::WithdrawalTransactions as u8],
        WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY.to_vec(),
    ]
}

/// Helper function to get the withdrawal transactions broadcasted path as [u8]
pub fn get_withdrawal_transactions_broadcasted_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions),
        &WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY,
    ]
}
