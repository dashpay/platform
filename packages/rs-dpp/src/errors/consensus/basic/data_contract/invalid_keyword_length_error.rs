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
#[error("Data contract {data_contract_id} has keyword with invalid length: '{keyword}'. Valid length is between 3 and 50 characters.")]
#[platform_serialize(unversioned)]
pub struct InvalidKeywordLengthError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    keyword: String,
}

impl InvalidKeywordLengthError {
    pub fn new(data_contract_id: Identifier, keyword: String) -> Self {
        Self {
            data_contract_id,
            keyword,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }

    pub fn keyword(&self) -> &str {
        &self.keyword
    }
}

impl From<InvalidKeywordLengthError> for ConsensusError {
    fn from(err: InvalidKeywordLengthError) -> Self {
        Self::BasicError(BasicError::InvalidKeywordLengthError(err))
    }
}
