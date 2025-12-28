use crate::drive::Drive;
use crate::drive::RootTree;

/// The subtree key for address balances storage
pub const ADDRESS_BALANCES_KEY: &[u8; 1] = b"a";

/// The subtree key for address balances storage as u8
pub const ADDRESS_BALANCES_KEY_U8: u8 = b'a';

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
}
