use crate::consensus::ConsensusError;
use crate::prelude::Identifier;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;
use crate::consensus::state::state_error::StateError;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid token release property mismatch for '{}', token ID: {}", property, token_id)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenReleasePropertyMismatch {
    property: String,
    token_id: Identifier,
}

impl InvalidTokenReleasePropertyMismatch {
    /// Creates a new `InvalidTokenReleasePropertyMismatch` error.
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

impl From<InvalidTokenReleasePropertyMismatch> for ConsensusError {
    fn from(err: InvalidTokenReleasePropertyMismatch) -> Self {
        Self::StateError(StateError::InvalidTokenReleasePropertyMismatch(err))
    }
}