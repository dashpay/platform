use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data contract {contract_id} has too many keywords: '{keywords_len}'. The maximum is 20.")]
#[platform_serialize(unversioned)]
pub struct TooManyKeywordsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: String,
    keywords_len: u8,
}

impl TooManyKeywordsError {
    pub fn new(contract_id: String, keywords_len: u8) -> Self {
        Self {
            contract_id,
            keywords_len,
        }
    }

    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    pub fn keywords_len(&self) -> &u8 {
        &self.keywords_len
    }
}

impl From<TooManyKeywordsError> for ConsensusError {
    fn from(err: TooManyKeywordsError) -> Self {
        Self::BasicError(BasicError::TooManyKeywordsError(err))
    }
}
