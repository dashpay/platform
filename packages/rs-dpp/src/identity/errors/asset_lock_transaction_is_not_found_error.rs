use dashcore::Txid;
use thiserror::Error;

use crate::DPPError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset Lock transaction {transaction_id:?} is not found")]
pub struct AssetLockTransactionIsNotFoundError {
    transaction_id: Txid,
}

impl AssetLockTransactionIsNotFoundError {
    pub fn new(transaction_id: Txid) -> Self {
        Self { transaction_id }
    }

    pub fn transaction_id(&self) -> Txid {
        self.transaction_id
    }
}

impl From<AssetLockTransactionIsNotFoundError> for DPPError {
    fn from(error: AssetLockTransactionIsNotFoundError) -> Self {
        Self::AssetLockTransactionIsNotFoundError(error)
    }
}
