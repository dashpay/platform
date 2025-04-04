use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::TokenContractPosition;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid token position {}, max {}",
    invalid_token_position,
    max_token_position
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenPositionError {
    max_token_position: TokenContractPosition,
    invalid_token_position: TokenContractPosition,
}

impl InvalidTokenPositionError {
    pub fn new(
        max_token_position: TokenContractPosition,
        invalid_token_position: TokenContractPosition,
    ) -> Self {
        Self {
            max_token_position,
            invalid_token_position,
        }
    }

    pub fn max_token_position(&self) -> TokenContractPosition {
        self.max_token_position
    }

    pub fn invalid_token_position(&self) -> TokenContractPosition {
        self.invalid_token_position
    }
}

impl From<InvalidTokenPositionError> for ConsensusError {
    fn from(err: InvalidTokenPositionError) -> Self {
        Self::BasicError(BasicError::InvalidTokenPositionError(err))
    }
}
