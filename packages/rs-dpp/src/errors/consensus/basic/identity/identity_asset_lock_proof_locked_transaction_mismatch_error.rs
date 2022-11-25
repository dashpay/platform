use dashcore::Txid;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("`Instant Lock transaction {instant_lock_transaction_id:?} and Asset lock transaction {asset_lock_transaction_id:?} mismatch`")]
pub struct IdentityAssetLockProofLockedTransactionMismatchError {
    instant_lock_transaction_id: Txid,
    asset_lock_transaction_id: Txid,
}

impl IdentityAssetLockProofLockedTransactionMismatchError {
    pub fn new(instant_lock_transaction_id: Txid, asset_lock_transaction_id: Txid) -> Self {
        Self {
            instant_lock_transaction_id,
            asset_lock_transaction_id,
        }
    }

    pub fn instant_lock_transaction_id(&self) -> Txid {
        self.instant_lock_transaction_id
    }

    pub fn asset_lock_transaction_id(&self) -> Txid {
        self.asset_lock_transaction_id
    }
}
