use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
#[error("Invalid shielded transaction proof: {message}")]
pub struct InvalidShieldedProofError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    message: String,
}

impl InvalidShieldedProofError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<InvalidShieldedProofError> for ConsensusError {
    fn from(err: InvalidShieldedProofError) -> Self {
        Self::StateError(StateError::InvalidShieldedProofError(err))
    }
}
