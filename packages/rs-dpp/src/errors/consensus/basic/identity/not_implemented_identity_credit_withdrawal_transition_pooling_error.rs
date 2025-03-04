use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "pooling {pooling:?} should be equal to 0. Other pooling mechanism are not implemented yet"
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct NotImplementedIdentityCreditWithdrawalTransitionPoolingError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub pooling: u8,
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
