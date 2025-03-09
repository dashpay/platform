use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("'{document_type}' document has more than '{index_limit}' unique indexes (contested is {is_contested_limit})")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct UniqueIndicesLimitReachedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub document_type: String,
    pub index_limit: u16,
    pub is_contested_limit: bool,
}

impl UniqueIndicesLimitReachedError {
    pub fn new(document_type: String, index_limit: u16, is_contested_limit: bool) -> Self {
        Self {
            document_type,
            index_limit,
            is_contested_limit,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }
    pub fn index_limit(&self) -> u16 {
        self.index_limit
    }

    pub fn is_contested_limit(&self) -> bool {
        self.is_contested_limit
    }
}

impl From<UniqueIndicesLimitReachedError> for ConsensusError {
    fn from(err: UniqueIndicesLimitReachedError) -> Self {
        Self::BasicError(BasicError::UniqueIndicesLimitReachedError(err))
    }
}
