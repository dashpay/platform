use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Document type '{document_type}' has a contested unique index '{contested_unique_index_name}' and a unique index '{unique_index_name}' as well which is not allowed"
)]
#[platform_serialize(unversioned)]
pub struct ContestedUniqueIndexWithUniqueIndexError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    contested_unique_index_name: String,
    unique_index_name: String,
}

impl ContestedUniqueIndexWithUniqueIndexError {
    pub fn new(
        document_type: String,
        contested_unique_index_name: String,
        unique_index_name: String,
    ) -> Self {
        Self {
            document_type,
            contested_unique_index_name,
            unique_index_name,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    pub fn contested_unique_index_name(&self) -> &str {
        &self.contested_unique_index_name
    }

    pub fn unique_index_name(&self) -> &str {
        &self.unique_index_name
    }
}

impl From<ContestedUniqueIndexWithUniqueIndexError> for ConsensusError {
    fn from(err: ContestedUniqueIndexWithUniqueIndexError) -> Self {
        Self::BasicError(BasicError::ContestedUniqueIndexWithUniqueIndexError(err))
    }
}
