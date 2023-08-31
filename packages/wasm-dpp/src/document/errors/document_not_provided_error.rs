// use crate::document::state_transition::document_batch_transition::document_transition::from_document_transition_to_js_value;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Document was not provided for apply of state transition")]
pub struct DocumentNotProvidedError {
    document_transition: DocumentTransition,
}

#[wasm_bindgen]
impl DocumentNotProvidedError {
    // #[wasm_bindgen(js_name=getDocumentTransition)]
    // pub fn get_document_transition(&self) -> JsValue {
    //     from_document_transition_to_js_value(self.document_transition.clone())
    // }
}

impl DocumentNotProvidedError {
    pub fn new(document_transition: DocumentTransition) -> DocumentNotProvidedError {
        Self {
            document_transition,
        }
    }
}
