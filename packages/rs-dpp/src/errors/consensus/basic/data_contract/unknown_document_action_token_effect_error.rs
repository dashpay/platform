use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Unrecognized document action token effect: allowed {:?}, got {}",
    allowed_values,
    received
)]
#[platform_serialize(unversioned)]
pub struct UnknownDocumentActionTokenEffectError {
    allowed_values: Vec<u8>,
    received: u64,
}

impl UnknownDocumentActionTokenEffectError {
    pub fn new(allowed_values: Vec<u8>, received: u64) -> Self {
        Self {
            allowed_values,
            received,
        }
    }

    pub fn allowed_values(&self) -> &[u8] {
        &self.allowed_values
    }

    pub fn received(&self) -> u64 {
        self.received
    }
}

impl From<UnknownDocumentActionTokenEffectError> for ConsensusError {
    fn from(err: UnknownDocumentActionTokenEffectError) -> Self {
        Self::BasicError(BasicError::UnknownDocumentActionTokenEffectError(err))
    }
}
