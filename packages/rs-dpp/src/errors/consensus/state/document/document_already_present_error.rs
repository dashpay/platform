use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Document {document_id} is already present")]
pub struct DocumentAlreadyPresentError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
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
