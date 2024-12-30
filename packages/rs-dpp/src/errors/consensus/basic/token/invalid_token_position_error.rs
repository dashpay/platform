use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use thiserror::Error;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use bincode::{Decode, Encode};
use crate::data_contract::TokenContractPosition;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid token position {}, expected {}", invalid_token_position, expected_token_position)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenPositionError {
    expected_token_position: TokenContractPosition,
    invalid_token_position: TokenContractPosition,
}

impl InvalidTokenPositionError {
    pub fn new(expected_token_position: TokenContractPosition, invalid_token_position: TokenContractPosition) -> Self {
        Self {
            expected_token_position,
            invalid_token_position,
        }
    }

    pub fn expected_token_position(&self) -> TokenContractPosition {
        self.expected_token_position
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