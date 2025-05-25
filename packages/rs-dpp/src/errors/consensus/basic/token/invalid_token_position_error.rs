use crate::data_contract::TokenContractPosition;
use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid token position: {invalid_token_position}. {max_token_message}",
    max_token_message = if let Some(max) = self.max_token_position {
        format!("The maximum allowed token position is {}", max)
    } else {
        "No maximum token position limit is set.".to_string()
    }
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidTokenPositionError {
    pub max_token_position: Option<TokenContractPosition>,
    pub invalid_token_position: TokenContractPosition,
}

impl InvalidTokenPositionError {
    pub fn new(
        max_token_position: Option<TokenContractPosition>,
        invalid_token_position: TokenContractPosition,
    ) -> Self {
        Self {
            max_token_position,
            invalid_token_position,
        }
    }

    pub fn max_token_position(&self) -> Option<TokenContractPosition> {
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
