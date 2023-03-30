use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Protocol version {parsed_protocol_version:?} is not supported. Latest supported version is {latest_version:?}")]
pub struct UnsupportedProtocolVersionError {
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

impl TryFrom<&UnsupportedProtocolVersionError> for Value {
    type Error = ProtocolError;

    fn try_from(value: &UnsupportedProtocolVersionError) -> Result<Self, Self::Error> {
        platform_value::to_value(value).map_err(ProtocolError::ValueError)
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
    use std::convert::TryInto;

    #[test]
    fn into_value() {
        let error = UnsupportedProtocolVersionError::new(1, 2);

        let value = Value::try_from(&error).expect("should convert to value");

        let recovered_error: UnsupportedProtocolVersionError =
            value.try_into().expect("should recover from value");

        assert_eq!(recovered_error, error);
    }
}
