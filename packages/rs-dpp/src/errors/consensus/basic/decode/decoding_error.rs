use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;
use crate::data_contract::errors::contract::DataContractError;
use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize)]
#[error("Decoding error: {error}")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct DecodingError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub error: String,
}

impl DecodingError {
    pub fn new(error: String) -> Self {
        Self { error }
    }

    pub fn error(&self) -> &str {
        &self.error
    }
}

impl From<DecodingError> for ConsensusError {
    fn from(err: DecodingError) -> Self {
        Self::BasicError(BasicError::ContractError(
            DataContractError::DecodingContractError(err),
        ))
    }
}

impl From<DecodingError> for DataContractError {
    fn from(err: DecodingError) -> Self {
        Self::DecodingContractError(err)
    }
}
