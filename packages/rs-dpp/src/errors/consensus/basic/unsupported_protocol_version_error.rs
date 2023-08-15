use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Protocol version {parsed_protocol_version:?} is not supported. Latest supported version is {latest_version:?}")]
pub struct UnsupportedProtocolVersionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    parsed_protocol_version: u32,
    latest_version: u32,
}

impl UnsupportedProtocolVersionError {
    pub fn new(parsed_protocol_version: u32, latest_version: u32) -> Self {
        Self {
            parsed_protocol_version,
            latest_version,
        }
    }

    pub fn parsed_protocol_version(&self) -> u32 {
        self.parsed_protocol_version
    }

    pub fn latest_version(&self) -> u32 {
        self.latest_version
    }
}

impl From<UnsupportedProtocolVersionError> for ConsensusError {
    fn from(err: UnsupportedProtocolVersionError) -> Self {
        Self::BasicError(BasicError::UnsupportedProtocolVersionError(err))
    }
}

impl TryFrom<Value> for UnsupportedProtocolVersionError {
    type Error = ProtocolError;

    fn try_from(args: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(args).map_err(ProtocolError::ValueError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion};
    use platform_version::version::LATEST_PLATFORM_VERSION;

    #[test]
    fn test_try_from() {
        let error = UnsupportedProtocolVersionError::new(1, 2);

        let consensus_error: ConsensusError = error.clone().into();

        let _cbor = consensus_error
            .serialize_with_platform_version(LATEST_PLATFORM_VERSION)
            .expect("should serialize");

        // let value = Value::try_from(&consensus_error).expect("should convert to value");

        // let recovered_error: UnsupportedProtocolVersionError =
        //     value.try_into().expect("should recover from value");
        //
        // assert_eq!(recovered_error, error);
    }
}
