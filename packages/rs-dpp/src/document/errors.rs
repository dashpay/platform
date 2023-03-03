use serde_json::Value;
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use super::document_transition::DocumentTransition;
use crate::document::{Document, ExtendedDocument};

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
    #[error("Invalid Document initial revision '{}'", document.revision().copied().unwrap_or_default())]
    InvalidInitialRevisionError { document: Box<ExtendedDocument> },

    #[error("Revision absent on mutable document")]
    RevisionAbsentError { document: Box<ExtendedDocument> },

    #[error("Trying To Replace Immutable Document")]
    TryingToReplaceImmutableDocument { document: Box<ExtendedDocument> },

    #[error("Documents have mixed owner ids")]
    MismatchOwnerIdsError { documents: Vec<ExtendedDocument> },

    #[error("No previous revision error")]
    DocumentNoRevisionError { document: Box<Document> },

    #[error("No documents were supplied to state transition")]
    NoDocumentsSuppliedError,
}
