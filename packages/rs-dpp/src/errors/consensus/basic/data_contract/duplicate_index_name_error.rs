use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Duplicate index name '{duplicate_index_name}' defined in '{document_type}' document")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct DuplicateIndexNameError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub document_type: String,
    pub duplicate_index_name: String,
}

impl DuplicateIndexNameError {
    pub fn new(document_type: String, duplicate_index_name: String) -> Self {
        Self {
            document_type,
            duplicate_index_name,
        }
    }

    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }

    pub fn duplicate_index_name(&self) -> String {
        self.duplicate_index_name.clone()
    }
}

impl From<DuplicateIndexNameError> for ConsensusError {
    fn from(err: DuplicateIndexNameError) -> Self {
        Self::BasicError(BasicError::DuplicateIndexNameError(err))
    }
}
