use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Incoherent parameters in token distribution function: {}", message)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenDistributionFunctionIncoherenceError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING A NEW VERSION

    */
    message: String,
}

impl InvalidTokenDistributionFunctionIncoherenceError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<InvalidTokenDistributionFunctionIncoherenceError> for ConsensusError {
    fn from(err: InvalidTokenDistributionFunctionIncoherenceError) -> Self {
        Self::BasicError(BasicError::InvalidTokenDistributionFunctionIncoherenceError(err))
    }
}
