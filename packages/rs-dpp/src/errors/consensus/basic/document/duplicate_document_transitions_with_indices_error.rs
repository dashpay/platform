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
    "Document transitions with duplicate unique properties: {:?}",
    references
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DuplicateDocumentTransitionsWithIndicesError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub references: Vec<(String, [u8; 32])>,
}

impl DuplicateDocumentTransitionsWithIndicesError {
    pub fn new(references: Vec<(String, [u8; 32])>) -> Self {
        Self { references }
    }

    pub fn references(&self) -> &Vec<(String, [u8; 32])> {
        &self.references
    }
}

impl From<DuplicateDocumentTransitionsWithIndicesError> for ConsensusError {
    fn from(err: DuplicateDocumentTransitionsWithIndicesError) -> Self {
        Self::BasicError(BasicError::DuplicateDocumentTransitionsWithIndicesError(
            err,
        ))
    }
}
