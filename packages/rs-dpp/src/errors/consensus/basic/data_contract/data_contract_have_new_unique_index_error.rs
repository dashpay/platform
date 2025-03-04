use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document with type {document_type} has a new unique index named '{index_name}'. Adding unique indices during Data Contract update is not allowed.")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DataContractHaveNewUniqueIndexError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub document_type: String,
    pub index_name: String,
}

impl DataContractHaveNewUniqueIndexError {
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

impl From<DataContractHaveNewUniqueIndexError> for ConsensusError {
    fn from(err: DataContractHaveNewUniqueIndexError) -> Self {
        Self::BasicError(BasicError::DataContractHaveNewUniqueIndexError(err))
    }
}
