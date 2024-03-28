use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use dashcore::Txid;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Asset lock transaction {transaction_id} output {output_index} already completely used")]
#[platform_serialize(unversioned)]
pub struct IdentityAssetLockTransactionOutPointAlreadyConsumedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[platform_serialize(with_serde)]
    #[bincode(with_serde)]
    transaction_id: Txid,
    output_index: usize,
}

impl IdentityAssetLockTransactionOutPointAlreadyConsumedError {
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

impl From<IdentityAssetLockTransactionOutPointAlreadyConsumedError> for ConsensusError {
    fn from(err: IdentityAssetLockTransactionOutPointAlreadyConsumedError) -> Self {
        Self::BasicError(BasicError::IdentityAssetLockTransactionOutPointAlreadyConsumedError(err))
    }
}
