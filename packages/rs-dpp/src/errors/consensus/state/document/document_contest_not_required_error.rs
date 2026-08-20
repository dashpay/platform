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
#[error("Document {document_id} prefunded the voting balance of index {provided_index_name}, but this document does not resolve to a contested index")]
#[platform_serialize(unversioned)]
pub struct DocumentContestNotRequiredError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,

    provided_index_name: String,
}

impl DocumentContestNotRequiredError {
    pub fn new(document_id: Identifier, provided_index_name: String) -> Self {
        Self {
            document_id,
            provided_index_name,
        }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn provided_index_name(&self) -> &str {
        &self.provided_index_name
    }
}

impl From<DocumentContestNotRequiredError> for ConsensusError {
    fn from(err: DocumentContestNotRequiredError) -> Self {
        Self::StateError(StateError::DocumentContestNotRequiredError(err))
    }
}
