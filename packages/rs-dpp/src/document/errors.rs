use serde_json::Value;
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use super::document_transition::DocumentTransition;
use super::Document;

#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("Document already exists")]
    DocumentAlreadyExistsError {
        document_transition: DocumentTransition,
    },
    #[error("Document was not provided for apply of state transition")]
    DocumentNotProvidedError {
        document_transition: DocumentTransition,
    },
    #[error("Invalid Document action submitted")]
    InvalidActionNameError { actions: Vec<String> },
    #[error("Invalid Document action '{}'", document_transition.base().action)]
    InvalidDocumentActionError {
        document_transition: DocumentTransition,
    },
    #[error("Invalid document: {}", errors[0])]
    InvalidDocumentError {
        errors: Vec<ConsensusError>,
        raw_document: Value,
    },
    #[error("Invalid Document initial revision '{}'", document.revision)]
    InvalidInitialRevisionError { document: Box<Document> },

    #[error("Documents have mixed owner ids")]
    MismatchOwnerIdsError { documents: Vec<Document> },

    #[error("No documents were supplied to state transition")]
    NoDocumentsSuppliedError,
}
