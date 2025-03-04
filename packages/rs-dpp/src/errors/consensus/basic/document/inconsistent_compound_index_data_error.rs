use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Unique compound index properties {:?} are partially set for {document_type}",
    index_properties
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InconsistentCompoundIndexDataError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub document_type: String,
    pub index_properties: Vec<String>,
}

impl InconsistentCompoundIndexDataError {
    pub fn new(document_type: String, index_properties: Vec<String>) -> Self {
        Self {
            document_type,
            index_properties,
        }
    }

    pub fn index_properties(&self) -> Vec<String> {
        self.index_properties.clone()
    }
    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }
}

impl From<InconsistentCompoundIndexDataError> for ConsensusError {
    fn from(err: InconsistentCompoundIndexDataError) -> Self {
        Self::BasicError(BasicError::InconsistentCompoundIndexDataError(err))
    }
}
