pub mod daily_withdrawal_limit;
#[cfg(feature = "withdrawals-contract")]
mod document_try_into_asset_unlock_base_transaction_info;

use bincode::{Decode, Encode};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[repr(u8)]
#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug, Encode, Decode, Default,
)]
pub enum Pooling {
    #[default]
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}

/// Transaction index type
pub type WithdrawalTransactionIndex = u64;

/// Simple type alias for withdrawal transaction with it's index
pub type WithdrawalTransactionIndexAndBytes = (WithdrawalTransactionIndex, Vec<u8>);
