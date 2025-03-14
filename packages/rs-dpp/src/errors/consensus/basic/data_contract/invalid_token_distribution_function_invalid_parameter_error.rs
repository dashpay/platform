use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid parameter `{}` in token distribution function. Expected range: {} to {}{}",
    parameter,
    min,
    max,
    if let Some(not_valid) = not_valid {
        format!(" except {} (which we got)", not_valid)
    } else {
        "".to_string()
    }
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidTokenDistributionFunctionInvalidParameterError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING A NEW VERSION

    */
    pub parameter: String,
    pub min: i64,
    pub max: i64,
    pub not_valid: Option<i64>,
}

impl InvalidTokenDistributionFunctionInvalidParameterError {
    pub fn new(parameter: String, min: i64, max: i64, not_valid: Option<i64>) -> Self {
        Self {
            parameter,
            min,
            max,
            not_valid,
        }
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

    pub fn not_valid(&self) -> Option<i64> {
        self.not_valid
    }
}

impl From<InvalidTokenDistributionFunctionInvalidParameterError> for ConsensusError {
    fn from(err: InvalidTokenDistributionFunctionInvalidParameterError) -> Self {
        Self::BasicError(BasicError::InvalidTokenDistributionFunctionInvalidParameterError(err))
    }
}
