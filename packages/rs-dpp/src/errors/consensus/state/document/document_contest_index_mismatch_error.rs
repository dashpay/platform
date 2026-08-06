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
#[error("Contest for document {document_id} prefunded the voting balance of index {provided_index_name}, but the contested index resolved for this document is {expected_index_name}")]
#[platform_serialize(unversioned)]
pub struct DocumentContestIndexMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,

    expected_index_name: String,

    provided_index_name: String,
}

impl DocumentContestIndexMismatchError {
    pub fn new(
        document_id: Identifier,
        expected_index_name: String,
        provided_index_name: String,
    ) -> Self {
        Self {
            document_id,
            expected_index_name,
            provided_index_name,
        }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn expected_index_name(&self) -> &str {
        &self.expected_index_name
    }

    pub fn provided_index_name(&self) -> &str {
        &self.provided_index_name
    }
}

impl From<DocumentContestIndexMismatchError> for ConsensusError {
    fn from(err: DocumentContestIndexMismatchError) -> Self {
        Self::StateError(StateError::DocumentContestIndexMismatchError(err))
    }
}
