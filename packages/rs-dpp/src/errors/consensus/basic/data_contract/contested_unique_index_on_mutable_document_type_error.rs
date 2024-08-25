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
    "Document type '{document_type}' has a contested unique index '{contested_unique_index_name}' but is set as mutable which is not allowed"
)]
#[platform_serialize(unversioned)]
pub struct ContestedUniqueIndexOnMutableDocumentTypeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    contested_unique_index_name: String,
}

impl ContestedUniqueIndexOnMutableDocumentTypeError {
    pub fn new(document_type: String, contested_unique_index_name: String) -> Self {
        Self {
            document_type,
            contested_unique_index_name,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    pub fn contested_unique_index_name(&self) -> &str {
        &self.contested_unique_index_name
    }
}

impl From<ContestedUniqueIndexOnMutableDocumentTypeError> for ConsensusError {
    fn from(err: ContestedUniqueIndexOnMutableDocumentTypeError) -> Self {
        Self::BasicError(BasicError::ContestedUniqueIndexOnMutableDocumentTypeError(
            err,
        ))
    }
}
