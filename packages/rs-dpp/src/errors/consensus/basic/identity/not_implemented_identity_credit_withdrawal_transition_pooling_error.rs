use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "pooling {pooling:?} should be equal to 0. Other pooling mechanism are not implemented yet"
)]
#[platform_serialize(unversioned)]
pub struct NotImplementedIdentityCreditWithdrawalTransitionPoolingError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pooling: u8,
}

impl NotImplementedIdentityCreditWithdrawalTransitionPoolingError {
    pub fn new(pooling: u8) -> Self {
        Self { pooling }
    }

    pub fn pooling(&self) -> u8 {
        self.pooling
    }
}

impl From<NotImplementedIdentityCreditWithdrawalTransitionPoolingError> for ConsensusError {
    fn from(err: NotImplementedIdentityCreditWithdrawalTransitionPoolingError) -> Self {
        Self::BasicError(
            BasicError::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(err),
        )
    }
}
