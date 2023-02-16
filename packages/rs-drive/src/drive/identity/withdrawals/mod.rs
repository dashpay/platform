/// Functions related to updating of a withdrawal status
pub mod documents;
/// Functions and constants related to GroveDB paths
pub mod paths;
/// Functions related to withdrawal queue
pub mod queue;
/// Functions related to transaction index counter
pub mod transaction_index;

/// Simple type alias for withdrawal transaction with it's id
pub type WithdrawalTransactionIdAndBytes = (Vec<u8>, Vec<u8>);
