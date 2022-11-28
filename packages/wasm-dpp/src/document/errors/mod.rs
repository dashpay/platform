use wasm_bindgen::prelude::*;

pub use document_already_exists_error::*;
pub use document_not_provided_error::*;
use dpp::document::errors::DocumentError;
pub use invalid_action_name_error::*;
pub use invalid_document_action_error::*;
pub use invalid_document_error::*;
pub use invalid_initial_revision_error::*;
pub use mismatch_owners_ids_error::*;
pub use no_documents_supplied_error::*;

use crate::mocks;
use crate::{utils::*, DocumentWasm};

mod document_already_exists_error;
mod document_not_provided_error;
mod invalid_action_name_error;
mod invalid_document_action_error;
mod invalid_document_error;
mod invalid_initial_revision_error;
mod mismatch_owners_ids_error;
mod no_documents_supplied_error;

pub fn from_document_to_js_error(e: DocumentError) -> JsValue {
    match e {
        DocumentError::DocumentAlreadyExists {
            document_transition,
        } => DocumentAlreadyExistsError::new(document_transition.into()).into(),
        DocumentError::DocumentNotProvided {
            document_transition,
        } => DocumentNotProvidedError::new(document_transition.into()).into(),

        DocumentError::InvalidActionName { actions } => {
            InvalidActionNameError::new(to_vec_js(actions)).into()
        }
        DocumentError::InvalidDocument { errors, document } => InvalidDocumentError::new(
            (*document).into(),
            errors
                .into_iter()
                .map(mocks::from_consensus_to_js_error)
                .collect(),
        )
        .into(),
        DocumentError::InvalidDocumentAction {
            document_transition,
        } => InvalidDocumentActionError::new(document_transition.into()).into(),
        DocumentError::InvalidInitialRevision { document } => {
            InvalidInitialRevisionError::new((*document).into()).into()
        }
        DocumentError::MismatchOwnersIds { documents } => {
            let documents_wasm: Vec<DocumentWasm> =
                documents.into_iter().map(DocumentWasm::from).collect();
            MismatchOwnersIdsError::new(to_vec_js(documents_wasm)).into()
        }
        DocumentError::NotDocumentsSupplied => NotDocumentsSuppliedError::new().into(),
    }
}
