use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use super::document_transition::DocumentTransition;
use super::Document;

#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("Document already exists")]
    DocumentAlreadyExists {
        document_transition: DocumentTransition,
    },
    #[error("Document was not provided for apply of state transition")]
    DocumentNotProvided {
        document_transition: DocumentTransition,
    },
    #[error("Invalid Document action submitted")]
    InvalidActionName { actions: Vec<String> },
    #[error("Invalid Document action '{}'", document_transition.base().action)]
    InvalidDocumentAction {
        document_transition: DocumentTransition,
    },
    #[error("Invalid document: {}", errors[0])]
    InvalidDocument {
        errors: Vec<ConsensusError>,
        document: Box<Document>,
    },
    #[error("Invalid Document Initial revision '{}'", document.revision)]
    InvalidInitialRevision { document: Box<Document> },

    #[error("Documents have mixed owner ids")]
    MismatchOwnersIds { documents: Vec<Document> },

    #[error("No documents were supplied to state transition")]
    NotDocumentsSupplied,
}
