/// Functions related to withdrawal documents
pub mod document;

/// Functions and constants related to GroveDB paths
pub mod paths;
/// Functions related to withdrawal transactions
pub mod transaction;

/// Simple type alias for withdrawal transaction with it's index
pub type WithdrawalTransactionIndexAndBytes = (WithdrawalTransactionIndex, Vec<u8>);
/// Transaction index type
pub type WithdrawalTransactionIndex = u64;
