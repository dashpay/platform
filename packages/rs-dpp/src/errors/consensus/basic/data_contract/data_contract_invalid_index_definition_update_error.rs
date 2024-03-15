use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document with type {document_type} could not add or remove '{index_path}' during data contract update as we do not allow modifications of data contract index paths")]
#[platform_serialize(unversioned)]
pub struct DataContractInvalidIndexDefinitionUpdateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_path: String,
}

impl DataContractInvalidIndexDefinitionUpdateError {
    pub fn new(document_type: String, index_name: String) -> Self {
        Self {
            document_type,
            index_path: index_name,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    pub fn index_path(&self) -> &str {
        &self.index_path
    }
}

impl From<DataContractInvalidIndexDefinitionUpdateError> for ConsensusError {
    fn from(err: DataContractInvalidIndexDefinitionUpdateError) -> Self {
        Self::BasicError(BasicError::DataContractInvalidIndexDefinitionUpdateError(
            err,
        ))
    }
}
