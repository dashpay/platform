use platform_value::Value;
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use crate::document::Document;
#[cfg(feature = "extended-document")]
use crate::document::ExtendedDocument;
#[cfg(feature = "state-transitions")]
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
#[cfg(feature = "state-transitions")]
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionMethodsV0;

#[derive(Error, Debug)]
pub enum DocumentError {
    #[cfg(feature = "state-transitions")]
    #[error("Document already exists")]
    DocumentAlreadyExistsError {
        document_transition: DocumentTransition,
    },
    #[cfg(feature = "state-transitions")]
    #[error("Document was not provided for apply of state transition")]
    DocumentNotProvidedError {
        document_transition: DocumentTransition,
    },
    #[error("Invalid Document action number {0}")]
    InvalidActionError(u8),
    #[error("Invalid Document action submitted")]
    InvalidActionNameError { actions: Vec<String> },
    #[cfg(feature = "state-transitions")]
    #[error("Invalid Document action '{}'", document_transition)]
    InvalidDocumentActionError {
        document_transition: DocumentTransition,
    },
    #[error("Invalid document: {}", errors[0])]
    InvalidDocumentError {
        errors: Vec<ConsensusError>,
        raw_document: Value,
    },

    #[cfg(feature = "extended-document")]
    #[error("Invalid Document initial revision '{}'", document.revision().copied().unwrap_or_default())]
    InvalidInitialRevisionError { document: Box<ExtendedDocument> },

    #[cfg(feature = "extended-document")]
    #[error("Revision absent on mutable document")]
    RevisionAbsentError { document: Box<ExtendedDocument> },

    #[cfg(feature = "extended-document")]
    #[error("Trying To Replace Immutable Document")]
    TryingToReplaceImmutableDocument { document: Box<ExtendedDocument> },

    #[cfg(feature = "extended-document")]
    #[error("Documents have mixed owner ids")]
    MismatchOwnerIdsError { documents: Vec<ExtendedDocument> },

    #[error("No previous revision error")]
    DocumentNoRevisionError { document: Box<Document> },

    #[error("No documents were supplied to state transition")]
    NoDocumentsSuppliedError,
}
