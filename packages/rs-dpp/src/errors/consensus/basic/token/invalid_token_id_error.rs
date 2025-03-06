use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::prelude::Identifier;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid token id {}, expected {}",
    invalid_token_id,
    expected_token_id
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenIdError {
    expected_token_id: Identifier,
    invalid_token_id: Identifier,
}

impl InvalidTokenIdError {
    pub fn new(expected_token_id: Identifier, invalid_token_id: Identifier) -> Self {
        Self {
            expected_token_id,
            invalid_token_id,
        }
    }

    pub fn expected_token_id(&self) -> Identifier {
        self.expected_token_id
    }

    pub fn invalid_token_id(&self) -> Identifier {
        self.invalid_token_id
    }
}

impl From<InvalidTokenIdError> for ConsensusError {
    fn from(err: InvalidTokenIdError) -> Self {
        Self::BasicError(BasicError::InvalidTokenIdError(err))
    }
}
