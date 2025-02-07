use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid parameter `{}` in token distribution function. Expected range: {} to {}",
    parameter,
    min,
    max
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenDistributionFunctionInvalidParameterError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING A NEW VERSION

    */
    parameter: String,
    min: i64,
    max: i64,
}

impl InvalidTokenDistributionFunctionInvalidParameterError {
    pub fn new(parameter: String, min: i64, max: i64) -> Self {
        Self { parameter, min, max }
    }

    pub fn parameter(&self) -> &str {
        &self.parameter
    }

    pub fn min(&self) -> i64 {
        self.min
    }

    pub fn max(&self) -> i64 {
        self.max
    }
}

impl From<InvalidTokenDistributionFunctionInvalidParameterError> for ConsensusError {
    fn from(err: InvalidTokenDistributionFunctionInvalidParameterError) -> Self {
        Self::BasicError(BasicError::InvalidTokenDistributionFunctionInvalidParameterError(err))
    }
}