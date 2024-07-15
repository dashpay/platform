use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::document::errors::document_no_revision_error::DocumentNoRevisionError;
use crate::document::errors::invalid_action_error::InvalidActionError;
use crate::document::errors::revision_absent_error::RevisionAbsentError;
use crate::document::errors::trying_to_delete_immutable_document_error::TryingToDeleteImmutableDocumentError;
use crate::document::errors::trying_to_replace_immutable_document_error::TryingToReplaceImmutableDocumentError;
pub use document_already_exists_error::*;
pub use document_not_provided_error::*;
use dpp::document::errors::DocumentError;
pub use invalid_action_name_error::*;
pub use invalid_document_action_error::*;
pub use invalid_document_error::*;
pub use invalid_initial_revision_error::*;
pub use mismatch_owners_ids_error::*;
pub use no_documents_supplied_error::*;

use crate::errors::consensus::consensus_error::from_consensus_error;
use crate::utils::*;

mod document_already_exists_error;
mod document_no_revision_error;
mod document_not_provided_error;
mod invalid_action_error;
mod invalid_action_name_error;
mod invalid_document_action_error;
mod invalid_document_error;
mod invalid_initial_revision_error;
mod mismatch_owners_ids_error;
mod no_documents_supplied_error;
mod revision_absent_error;
mod trying_to_delete_immutable_document_error;
mod trying_to_replace_immutable_document_error;

pub fn from_document_to_js_error(e: DocumentError) -> JsValue {
    match e {
        DocumentError::DocumentAlreadyExistsError {
            document_transition,
        } => DocumentAlreadyExistsError::new(document_transition).into(),
        DocumentError::DocumentNotProvidedError {
            document_transition,
        } => DocumentNotProvidedError::new(document_transition).into(),

        DocumentError::InvalidActionNameError { actions } => {
            InvalidActionNameError::new(to_vec_js(actions)).into()
        }
        DocumentError::InvalidDocumentError {
            errors,
            raw_document,
        } => InvalidDocumentError::new(
            raw_document
                .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
                .expect("Raw Document should be serializable into JsValue"),
            errors.into_iter().map(from_consensus_error).collect(),
        )
        .into(),
        DocumentError::InvalidDocumentActionError {
            document_transition,
        } => InvalidDocumentActionError::new(document_transition).into(),
        DocumentError::InvalidInitialRevisionError { document } => {
            InvalidInitialRevisionError::new((*document).into()).into()
        }
        DocumentError::MismatchOwnerIdsError { documents } => {
            MismatchOwnerIdsError::from_documents(documents).into()
        }
        DocumentError::NoDocumentsSuppliedError => NoDocumentsSuppliedError::new().into(),
        DocumentError::DocumentNoRevisionError { document } => {
            DocumentNoRevisionError::new((*document).into()).into()
        }
        DocumentError::RevisionAbsentError { document } => {
            RevisionAbsentError::new((*document).into()).into()
        }
        DocumentError::TryingToReplaceImmutableDocument { document } => {
            TryingToReplaceImmutableDocumentError::new((*document).into()).into()
        }
        DocumentError::InvalidActionError(action) => InvalidActionError::new(action.into()).into(),
        DocumentError::TryingToDeleteIndelibleDocument { document } => {
            TryingToDeleteImmutableDocumentError::new((*document).into()).into()
        }
    }
}
