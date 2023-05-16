use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Document {document_id} createdAt and updatedAt should not be equal")]
pub struct DocumentTimestampsAreEqualError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
}

impl DocumentTimestampsAreEqualError {
    pub fn new(document_id: Identifier) -> Self {
        Self { document_id }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }
}

impl From<DocumentTimestampsAreEqualError> for ConsensusError {
    fn from(err: DocumentTimestampsAreEqualError) -> Self {
        Self::StateError(StateError::DocumentTimestampsAreEqualError(err))
    }
}
