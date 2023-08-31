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
#[error("Document {document_id} createdAt and updatedAt timestamps are not equal")]
#[platform_serialize(unversioned)]
pub struct DocumentTimestampsMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
}

impl DocumentTimestampsMismatchError {
    pub fn new(document_id: Identifier) -> Self {
        Self { document_id }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }
}

impl From<DocumentTimestampsMismatchError> for ConsensusError {
    fn from(err: DocumentTimestampsMismatchError) -> Self {
        Self::StateError(StateError::DocumentTimestampsMismatchError(err))
    }
}
