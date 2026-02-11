use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid shielded value_balance: {message}")]
#[platform_serialize(unversioned)]
pub struct ShieldedInvalidValueBalanceError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    message: String,
}

impl ShieldedInvalidValueBalanceError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<ShieldedInvalidValueBalanceError> for ConsensusError {
    fn from(err: ShieldedInvalidValueBalanceError) -> Self {
        Self::BasicError(BasicError::ShieldedInvalidValueBalanceError(err))
    }
}
