use crate::drive::Drive;
use crate::drive::RootTree;

/// The subtree key for address balances storage
pub const ADDRESS_BALANCES_KEY: &[u8; 1] = b"m";

/// The subtree key for address balances storage as u8
pub const ADDRESS_BALANCES_KEY_U8: u8 = b'm';

/// The subtree key for address balances storage
pub const COMPACTED_ADDRESS_BALANCES_KEY: &[u8; 1] = b"c";

/// The subtree key for compacted address balances storage as u8
pub const COMPACTED_ADDRESS_BALANCES_KEY_U8: u8 = b'c';

/// The subtree key for compacted addresses expiration time storage
pub const COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY: &[u8; 1] = b"e";

/// The subtree key for compacted addresses expiration time storage as u8
pub const COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8: u8 = b'e';


impl Drive {
    /// Path to saved block transactions storage.
    pub fn saved_block_transactions_path() -> Vec<Vec<u8>> {
        vec![vec![RootTree::SavedBlockTransactions as u8]]
    }

    /// Path to address balances under saved block transactions.
    pub fn saved_block_transactions_address_balances_path_vec() -> Vec<Vec<u8>> {
        vec![
            vec![RootTree::SavedBlockTransactions as u8],
            vec![ADDRESS_BALANCES_KEY_U8],
        ]
    }

    /// Path to address balances under saved block transactions.
    pub fn saved_block_transactions_address_balances_path() -> [&'static [u8]; 2] {
        [
            Into::<&[u8; 1]>::into(RootTree::SavedBlockTransactions),
            &[ADDRESS_BALANCES_KEY_U8],
        ]
    }

    /// Path to compacted address balances under saved block transactions.
    pub fn saved_compacted_block_transactions_address_balances_path_vec() -> Vec<Vec<u8>> {
        vec![
            vec![RootTree::SavedBlockTransactions as u8],
            vec![COMPACTED_ADDRESS_BALANCES_KEY_U8],
        ]
    }

    /// Path to compacted address balances under saved block transactions.
    pub fn saved_compacted_block_transactions_address_balances_path() -> [&'static [u8]; 2] {
        [
            Into::<&[u8; 1]>::into(RootTree::SavedBlockTransactions),
            &[COMPACTED_ADDRESS_BALANCES_KEY_U8],
        ]
    }

    /// Path to compacted addresses expiration time under saved block transactions (Vec version).
    pub fn saved_compacted_addresses_expiration_time_path_vec() -> Vec<Vec<u8>> {
        vec![
            vec![RootTree::SavedBlockTransactions as u8],
            vec![COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8],
        ]
    }

    /// Path to compacted addresses expiration time under saved block transactions.
    pub fn saved_compacted_addresses_expiration_time_path() -> [&'static [u8]; 2] {
        [
            Into::<&[u8; 1]>::into(RootTree::SavedBlockTransactions),
            &[COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8],
        ]
    }

}
