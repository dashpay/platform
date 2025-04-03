use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data contract {contract_id} has invalid keyword '{keyword}'. Keywords must be ASCII.")]
#[platform_serialize(unversioned)]
pub struct InvalidKeywordEncodingError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: String,
    keyword: String,
}

impl InvalidKeywordEncodingError {
    pub fn new(contract_id: String, keyword: String) -> Self {
        Self {
            contract_id,
            keyword,
        }
    }

    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    pub fn keyword(&self) -> &str {
        &self.keyword
    }
}

impl From<InvalidKeywordEncodingError> for ConsensusError {
    fn from(err: InvalidKeywordEncodingError) -> Self {
        Self::BasicError(BasicError::InvalidKeywordEncodingError(err))
    }
}
