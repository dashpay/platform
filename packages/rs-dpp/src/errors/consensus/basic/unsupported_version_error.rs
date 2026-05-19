use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("version {received_version:?} is not supported. Supported versions are {min_version:?} to {max_version:?}")]
#[platform_serialize(unversioned)]
pub struct UnsupportedVersionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    received_version: u16,
    min_version: u16,
    max_version: u16,
}

impl UnsupportedVersionError {
    pub fn new(received_version: u16, min_version: u16, max_version: u16) -> Self {
        Self {
            received_version,
            min_version,
            max_version,
        }
    }

    pub fn received_version(&self) -> u16 {
        self.received_version
    }

    pub fn min_version(&self) -> u16 {
        self.min_version
    }

    pub fn max_version(&self) -> u16 {
        self.max_version
    }
}

impl From<UnsupportedVersionError> for ConsensusError {
    fn from(err: UnsupportedVersionError) -> Self {
        Self::BasicError(BasicError::UnsupportedVersionError(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::PlatformDeserializable;
    use crate::serialization::PlatformSerializableWithPlatformVersion;
    use platform_version::version::LATEST_PLATFORM_VERSION;

    #[test]
    fn test_try_from() {
        let error = UnsupportedVersionError::new(1, 2, 3);

        let consensus_error: ConsensusError = error.clone().into();

        let serialized_bytes = consensus_error
            .serialize_to_bytes_with_platform_version(LATEST_PLATFORM_VERSION)
            .expect("expected to serialize");

        let recovered_consensus_error =
            ConsensusError::deserialize_from_bytes(&serialized_bytes).expect("should deserialize");

        assert_eq!(consensus_error, recovered_consensus_error);
    }
}
