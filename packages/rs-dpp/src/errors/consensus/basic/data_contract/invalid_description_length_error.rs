use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data contract {} has description with invalid length: '{}'. Valid length is between 3 and 100 characters.", data_contract_id, description.len())]
#[platform_serialize(unversioned)]
pub struct InvalidDescriptionLengthError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    description: String,
}

impl InvalidDescriptionLengthError {
    pub fn new(data_contract_id: Identifier, description: String) -> Self {
        Self {
            data_contract_id,
            description,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
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
