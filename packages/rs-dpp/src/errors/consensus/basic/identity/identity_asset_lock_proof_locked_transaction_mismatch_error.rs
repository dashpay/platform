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
#[error("`Instant Lock transaction {instant_lock_transaction_id:?} and Asset lock transaction {asset_lock_transaction_id:?} mismatch`")]
#[platform_serialize(unversioned)]
pub struct IdentityAssetLockProofLockedTransactionMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[platform_serialize(with_serde)]
    #[bincode(with_serde)]
    instant_lock_transaction_id: Txid,
    #[platform_serialize(with_serde)]
    #[bincode(with_serde)]
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
impl From<IdentityAssetLockProofLockedTransactionMismatchError> for ConsensusError {
    fn from(err: IdentityAssetLockProofLockedTransactionMismatchError) -> Self {
        Self::BasicError(BasicError::IdentityAssetLockProofLockedTransactionMismatchError(err))
    }
}
