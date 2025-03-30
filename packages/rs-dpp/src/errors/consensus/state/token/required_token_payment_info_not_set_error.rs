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
#[error(
    "Required token payment info not set on token {} (action: {})",
    token_id,
    action
)]
#[platform_serialize(unversioned)]
pub struct RequiredTokenPaymentInfoNotSetError {
    token_id: Identifier,
    action: String,
}

impl RequiredTokenPaymentInfoNotSetError {
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

impl From<RequiredTokenPaymentInfoNotSetError> for ConsensusError {
    fn from(err: RequiredTokenPaymentInfoNotSetError) -> Self {
        Self::StateError(StateError::RequiredTokenPaymentInfoNotSetError(err))
    }
}
