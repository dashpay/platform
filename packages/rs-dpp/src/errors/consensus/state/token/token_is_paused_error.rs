use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Token {} is paused.", token_id)]
#[platform_serialize(unversioned)]
pub struct TokenIsPausedError {
    token_id: Identifier,
}

impl TokenIsPausedError {
    pub fn new(token_id: Identifier) -> Self {
        Self { token_id }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }
}

impl From<TokenIsPausedError> for ConsensusError {
    fn from(err: TokenIsPausedError) -> Self {
        Self::StateError(StateError::TokenIsPausedError(err))
    }
}
