use crate::prelude::Identifier;
use thiserror::Error;

#[derive(Error, Clone, Debug)]
pub enum StateError {
    #[error("Document {document_id} is already present")]
    DocumentAlreadyPresentError { document_id: Identifier },

    #[error("{document_id} document not found")]
    DocumentNotFoundError { document_id: Identifier },

    #[error("Provided document {document_id} owner ID {document_owner_id} mismatch with existing {existing_document_owner_id}")]
    DocumentOwnerMismatchError {
        document_id: Identifier,
        document_owner_id: Identifier,
        existing_document_owner_id: Identifier,
    },

    #[error("Document {document_id} createdAt and updatedAt timestamps are not equal")]
    DocumentTimestampMismatchError { document_id: Identifier },

    #[error("Document {document_id} {timestamp_name} timestamp {timestamp} are out of block time window from {time_window_start} and {time_window_end}")]
    DocumentTimestampWindowViolationError {
        timestamp_name: String,
        document_id: Identifier,
        timestamp: i64,
        time_window_start: i64,
        time_window_end: i64,
    },

    #[error("Document {document_id} has duplicate unique properties {duplicating_properties:?} with other documents")]
    DuplicateUniqueIndexError {
        document_id: Identifier,
        duplicating_properties: Vec<String>,
    },

    #[error(
        "Document {document_id} has invalid revision. The current revision is {current_revision}"
    )]
    InvalidDocumentRevisionError {
        document_id: Identifier,
        current_revision: u32,
    },
}
