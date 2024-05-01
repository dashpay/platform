use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document transitions with duplicate IDs {:?}", references)]
#[platform_serialize(unversioned)]
pub struct DuplicateDocumentTransitionsWithIdsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    references: Vec<(String, [u8; 32])>,
}

impl DuplicateDocumentTransitionsWithIdsError {
    pub fn new(references: Vec<(String, [u8; 32])>) -> Self {
        Self { references }
    }

    pub fn references(&self) -> &Vec<(String, [u8; 32])> {
        &self.references
    }
}

impl From<DuplicateDocumentTransitionsWithIdsError> for ConsensusError {
    fn from(err: DuplicateDocumentTransitionsWithIdsError) -> Self {
        Self::BasicError(BasicError::DuplicateDocumentTransitionsWithIdsError(err))
    }
}
