/// Functions related to updating of a withdrawal status
pub mod documents;
/// Functions and constants related to GroveDB paths
pub mod paths;
/// Functions related to withdrawal queue
// TODO: Rename to transaction_queue
pub mod queue;
/// Functions related to transaction index counter
pub mod transaction_index;

/// Simple type alias for withdrawal transaction with it's index
pub type WithdrawalTransactionIndexAndBytes = (WithdrawalTransactionIndex, Vec<u8>);
/// Transaction index type
pub type WithdrawalTransactionIndex = u64;
