use crate::consensus::state::state_error::StateError;
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
    "Invalid token release property mismatch for '{}', token ID: {}",
    property,
    token_id
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenClaimPropertyMismatch {
    property: String,
    token_id: Identifier,
}

impl InvalidTokenClaimPropertyMismatch {
    /// Creates a new `InvalidTokenClaimPropertyMismatch` error.
    pub fn new(property: impl Into<String>, token_id: Identifier) -> Self {
        Self {
            property: property.into(),
            token_id,
        }
    }

    /// Returns the property name that caused the mismatch.
    pub fn property(&self) -> &str {
        &self.property
    }

    /// Returns the token ID associated with the mismatch.
    pub fn token_id(&self) -> Identifier {
        self.token_id
    }
}

impl From<InvalidTokenClaimPropertyMismatch> for ConsensusError {
    fn from(err: InvalidTokenClaimPropertyMismatch) -> Self {
        Self::StateError(StateError::InvalidTokenClaimPropertyMismatch(err))
    }
}
