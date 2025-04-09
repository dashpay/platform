use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data contract {} has description with invalid length: '{}'. Valid length is between 3 and 100 characters.", contract_id, description.len())]
#[platform_serialize(unversioned)]
pub struct InvalidDescriptionLengthError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: String,
    description: String,
}

impl InvalidDescriptionLengthError {
    pub fn new(contract_id: String, description: String) -> Self {
        Self {
            contract_id,
            description,
        }
    }

    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

impl From<InvalidDescriptionLengthError> for ConsensusError {
    fn from(err: InvalidDescriptionLengthError) -> Self {
        Self::BasicError(BasicError::InvalidDescriptionLengthError(err))
    }
}
