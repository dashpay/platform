use dashcore::Txid;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("`Chain Locked transaction {transaction_id:?} could not be validated for the given height {height_reported_not_locked}`")]
pub struct InvalidIdentityAssetLockProofChainLockValidationError {
    transaction_id: Txid,
    height_reported_not_locked: u32,
}

impl InvalidIdentityAssetLockProofChainLockValidationError {
    pub fn new(transaction_id: Txid, height_reported_not_locked: u32) -> Self {
        Self {
            transaction_id,
            height_reported_not_locked,
        }
    }

    pub fn transaction_id(&self) -> Txid {
        self.transaction_id
    }

    pub fn height_reported_not_locked(&self) -> u32 {
        self.height_reported_not_locked
    }
}
