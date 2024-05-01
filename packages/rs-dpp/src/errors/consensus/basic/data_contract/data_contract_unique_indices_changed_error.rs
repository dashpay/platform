use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document with type {document_type} has updated unique index named '{index_name}'. Change of unique indices is not allowed")]
#[platform_serialize(unversioned)]
pub struct DataContractUniqueIndicesChangedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_name: String,
}

impl DataContractUniqueIndicesChangedError {
    pub fn new(document_type: String, index_name: String) -> Self {
        Self {
            document_type,
            index_name,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    pub fn index_name(&self) -> &str {
        &self.index_name
    }
}

impl From<DataContractUniqueIndicesChangedError> for ConsensusError {
    fn from(err: DataContractUniqueIndicesChangedError) -> Self {
        Self::BasicError(BasicError::DataContractUniqueIndicesChangedError(err))
    }
}
