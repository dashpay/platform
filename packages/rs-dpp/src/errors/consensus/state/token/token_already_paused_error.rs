use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Token {} is already paused. Action attempted: {}", token_id, action)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct TokenAlreadyPausedError {
    pub token_id: Identifier,
    pub action: String,
}

impl TokenAlreadyPausedError {
    pub fn new(token_id: Identifier, action: String) -> Self {
        Self { token_id, action }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn action(&self) -> &str {
        &self.action
    }
}

impl From<TokenAlreadyPausedError> for ConsensusError {
    fn from(err: TokenAlreadyPausedError) -> Self {
        Self::StateError(StateError::TokenAlreadyPausedError(err))
    }
}
