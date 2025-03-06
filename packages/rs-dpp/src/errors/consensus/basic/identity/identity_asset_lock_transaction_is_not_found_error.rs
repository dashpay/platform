use bincode::{Decode, Encode};
use std::fmt::{Display, Formatter};

use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
pub struct IdentityAssetLockTransactionIsNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    transaction_id: [u8; 32],
}

impl Display for IdentityAssetLockTransactionIsNotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let hex = hex::encode(self.transaction_id);
        let message = format!("Asset Lock transaction {hex} is not found");
        f.write_str(&message)
    }
}

impl IdentityAssetLockTransactionIsNotFoundError {
    pub fn new(transaction_id: [u8; 32]) -> Self {
        Self { transaction_id }
    }

    pub fn transaction_id(&self) -> &[u8; 32] {
        &self.transaction_id
    }
}

impl From<IdentityAssetLockTransactionIsNotFoundError> for ConsensusError {
    fn from(err: IdentityAssetLockTransactionIsNotFoundError) -> Self {
        Self::BasicError(BasicError::IdentityAssetLockTransactionIsNotFoundError(err))
    }
}
