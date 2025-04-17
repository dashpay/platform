use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Missing english ('en') localization which is using by default")]
#[platform_serialize(unversioned)]
pub struct MissingDefaultLocalizationError {}

impl Default for MissingDefaultLocalizationError {
    fn default() -> Self {
        Self::new()
    }
}

impl MissingDefaultLocalizationError {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<MissingDefaultLocalizationError> for ConsensusError {
    fn from(err: MissingDefaultLocalizationError) -> Self {
        Self::BasicError(BasicError::MissingDefaultLocalizationError(err))
    }
}
