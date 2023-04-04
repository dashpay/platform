use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use dashcore::Txid;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Asset lock transaction ${transaction_id:?} output ${output_index:?} already used")]
pub struct IdentityAssetLockTransactionOutPointAlreadyExistsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
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

impl From<IdentityAssetLockTransactionOutPointAlreadyExistsError> for ConsensusError {
    fn from(err: IdentityAssetLockTransactionOutPointAlreadyExistsError) -> Self {
        Self::BasicError(BasicError::IdentityAssetLockTransactionOutPointAlreadyExistsError(err))
    }
}
