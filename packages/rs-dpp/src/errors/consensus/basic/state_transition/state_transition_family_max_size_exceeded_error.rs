use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

/// A state transition of a family with its own size cap (the contract-code capable
/// generations of the contract create and update transitions) exceeded that cap.
///
/// Ordinary families keep reporting `StateTransitionMaxSizeExceededError`; this error names the
/// family so the message states which cap applied.
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("{family} state transition size {actual_size_bytes} is more than the family maximum {max_size_bytes}")]
#[platform_serialize(unversioned)]
pub struct StateTransitionFamilyMaxSizeExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    family: String,
    actual_size_bytes: u64,
    max_size_bytes: u64,
}

impl StateTransitionFamilyMaxSizeExceededError {
    pub fn new(family: impl Into<String>, actual_size_bytes: u64, max_size_bytes: u64) -> Self {
        Self {
            family: family.into(),
            actual_size_bytes,
            max_size_bytes,
        }
    }

    pub fn family(&self) -> &str {
        &self.family
    }

    pub fn actual_size_bytes(&self) -> u64 {
        self.actual_size_bytes
    }

    pub fn max_size_bytes(&self) -> u64 {
        self.max_size_bytes
    }
}

impl From<StateTransitionFamilyMaxSizeExceededError> for ConsensusError {
    fn from(err: StateTransitionFamilyMaxSizeExceededError) -> Self {
        Self::BasicError(BasicError::StateTransitionFamilyMaxSizeExceededError(err))
    }
}
