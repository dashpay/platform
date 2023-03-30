use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Document {document_id} is already present")]
pub struct DocumentAlreadyPresentError {
    document_id: Identifier,
}

impl DocumentAlreadyPresentError {
    pub fn new(document_id: Identifier) -> Self {
        Self { document_id }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }
}

impl From<DocumentAlreadyPresentError> for ConsensusError {
    fn from(err: DocumentAlreadyPresentError) -> Self {
        Self::StateError(StateError::DocumentAlreadyPresentError(err))
    }
}
