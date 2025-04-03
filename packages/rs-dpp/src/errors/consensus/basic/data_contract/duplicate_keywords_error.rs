use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data contract {contract_id} has duplicated keyword '{keyword}'.")]
#[platform_serialize(unversioned)]
pub struct DuplicateKeywordsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: String,
    keyword: String,
}

impl DuplicateKeywordsError {
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

impl From<DuplicateKeywordsError> for ConsensusError {
    fn from(err: DuplicateKeywordsError) -> Self {
        Self::BasicError(BasicError::DuplicateKeywordsError(err))
    }
}
