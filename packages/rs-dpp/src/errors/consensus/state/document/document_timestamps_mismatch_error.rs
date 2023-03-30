use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Document {document_id} createdAt and updatedAt timestamps are not equal")]
pub struct DocumentTimestampsMismatchError {
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
