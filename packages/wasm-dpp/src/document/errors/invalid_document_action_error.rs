use crate::document::state_transition::document_batch_transition::document_transition::from_document_transition_to_js_value;
use dpp::prelude::DocumentTransition;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document action: '{}'", document_transition.action())]
pub struct InvalidDocumentActionError {
    document_transition: DocumentTransition,
}

#[wasm_bindgen]
impl InvalidDocumentActionError {
    #[wasm_bindgen(js_name=getDocumentTransition)]
    pub fn get_document_transition(&self) -> JsValue {
        from_document_transition_to_js_value(self.document_transition.clone())
    }
}

impl InvalidDocumentActionError {
    pub fn new(document_transition: DocumentTransition) -> Self {
        Self {
            document_transition,
        }
    }
}
