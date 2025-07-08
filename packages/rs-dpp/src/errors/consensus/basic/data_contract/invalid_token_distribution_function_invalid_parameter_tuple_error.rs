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
    "Invalid parameter tuple in token distribution function: `{}` must be {} `{}`",
    first_parameter,
    relation,
    second_parameter
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenDistributionFunctionInvalidParameterTupleError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING A NEW VERSION

    */
    first_parameter: String,
    second_parameter: String,
    relation: String, // "greater than" or "smaller than"
}

impl InvalidTokenDistributionFunctionInvalidParameterTupleError {
    pub fn new(first_parameter: String, second_parameter: String, relation: String) -> Self {
        Self {
            first_parameter,
            second_parameter,
            relation,
        }
    }

    pub fn first_parameter(&self) -> &str {
        &self.first_parameter
    }

    pub fn second_parameter(&self) -> &str {
        &self.second_parameter
    }

    pub fn relation(&self) -> &str {
        &self.relation
    }
}

impl From<InvalidTokenDistributionFunctionInvalidParameterTupleError> for ConsensusError {
    fn from(err: InvalidTokenDistributionFunctionInvalidParameterTupleError) -> Self {
        Self::BasicError(
            BasicError::InvalidTokenDistributionFunctionInvalidParameterTupleError(err),
        )
    }
}
