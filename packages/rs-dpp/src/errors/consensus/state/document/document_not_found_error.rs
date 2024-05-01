use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("{document_id} document not found")]
#[platform_serialize(unversioned)]
pub struct DocumentNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
}

impl DocumentNotFoundError {
    pub fn new(document_id: Identifier) -> Self {
        Self { document_id }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }
}

impl From<DocumentNotFoundError> for ConsensusError {
    fn from(err: DocumentNotFoundError) -> Self {
        Self::StateError(StateError::DocumentNotFoundError(err))
    }
}
