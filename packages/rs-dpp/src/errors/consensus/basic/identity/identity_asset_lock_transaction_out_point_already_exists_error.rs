use dashcore::Txid;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset lock transaction ${transaction_id:?} output ${output_index:?} already used")]
pub struct IdentityAssetLockTransactionOutPointAlreadyExistsError {
    transaction_id: Txid,
    output_index: usize,
}

impl IdentityAssetLockTransactionOutPointAlreadyExistsError {
    pub fn new(transaction_id: Txid, output_index: usize) -> Self {
        Self {
            transaction_id,
            output_index,
        }
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }

    pub fn transaction_id(&self) -> Txid {
        self.transaction_id
    }
}
