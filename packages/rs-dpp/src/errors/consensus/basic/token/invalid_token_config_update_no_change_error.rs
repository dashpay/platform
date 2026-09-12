use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    DecodeUntrusted,
)]
#[error("Invalid token configuration update: no changes were made")]
#[platform_serialize(unversioned)]
pub struct InvalidTokenConfigUpdateNoChangeError;

impl Default for InvalidTokenConfigUpdateNoChangeError {
    fn default() -> Self {
        Self::new()
    }
}

impl InvalidTokenConfigUpdateNoChangeError {
    /// Creates a new `InvalidTokenConfigUpdateError`.
    pub fn new() -> Self {
        Self
    }
}

impl From<InvalidTokenConfigUpdateNoChangeError> for ConsensusError {
    fn from(err: InvalidTokenConfigUpdateNoChangeError) -> Self {
        Self::BasicError(BasicError::InvalidTokenConfigUpdateNoChangeError(err))
    }
}
