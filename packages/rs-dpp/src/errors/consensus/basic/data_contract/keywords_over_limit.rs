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
#[error(
    "Data contract {data_contract_id} has too many keywords: '{keywords_len}'. The maximum is 50."
)]
#[platform_serialize(unversioned)]
pub struct TooManyKeywordsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    keywords_len: u8,
}

impl TooManyKeywordsError {
    pub fn new(data_contract_id: Identifier, keywords_len: u8) -> Self {
        Self {
            data_contract_id,
            keywords_len,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
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
