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
#[error("Data contract {data_contract_id} has a keyword with invalid characters. Keywords must not contain whitespace or control characters.")]
#[platform_serialize(unversioned)]
pub struct InvalidKeywordCharacterError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    keyword: String,
}

impl InvalidKeywordCharacterError {
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

impl From<InvalidKeywordCharacterError> for ConsensusError {
    fn from(err: InvalidKeywordCharacterError) -> Self {
        Self::BasicError(BasicError::InvalidKeywordCharacterError(err))
    }
}
