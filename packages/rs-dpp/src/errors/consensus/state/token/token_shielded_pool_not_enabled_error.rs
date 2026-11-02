use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A shielded token operation targeted a token whose configuration has no shielded pool.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("Token {} does not have a shielded pool.", token_id)]
#[platform_serialize(unversioned)]
pub struct TokenShieldedPoolNotEnabledError {
    token_id: Identifier,
}

impl TokenShieldedPoolNotEnabledError {
    pub fn new(token_id: Identifier) -> Self {
        Self { token_id }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }
}

impl From<TokenShieldedPoolNotEnabledError> for ConsensusError {
    fn from(err: TokenShieldedPoolNotEnabledError) -> Self {
        Self::StateError(StateError::TokenShieldedPoolNotEnabledError(err))
    }
}
