use crate::consensus::basic::BasicError;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid token distribution function: division by zero in {}",
    distribution_function
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenDistributionFunctionDivideByZeroError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING A NEW VERSION

    */
    distribution_function: DistributionFunction,
}

impl InvalidTokenDistributionFunctionDivideByZeroError {
    pub fn new(distribution_function: DistributionFunction) -> Self {
        Self {
            distribution_function,
        }
    }

    pub fn distribution_function(&self) -> &DistributionFunction {
        &self.distribution_function
    }
}

impl From<InvalidTokenDistributionFunctionDivideByZeroError> for ConsensusError {
    fn from(err: InvalidTokenDistributionFunctionDivideByZeroError) -> Self {
        Self::BasicError(BasicError::InvalidTokenDistributionFunctionDivideByZeroError(err))
    }
}
