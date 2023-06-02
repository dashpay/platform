use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::prelude::Revision;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error(
    "Document {document_id} has invalid revision. The current revision is {current_revision:?}"
)]
pub struct InvalidDocumentRevisionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    current_revision: Option<Revision>,
}

impl InvalidDocumentRevisionError {
    pub fn new(document_id: Identifier, current_revision: Option<Revision>) -> Self {
        Self {
            document_id,
            current_revision,
        }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn current_revision(&self) -> &Option<Revision> {
        &self.current_revision
    }
}

impl From<InvalidDocumentRevisionError> for ConsensusError {
    fn from(err: InvalidDocumentRevisionError) -> Self {
        Self::StateError(StateError::InvalidDocumentRevisionError(err))
    }
}
