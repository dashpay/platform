use thiserror::Error;

use crate::mocks::DocumentTransitionWasm;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document action: '{}'", document_transition.get_action())]
pub struct InvalidDocumentActionError {
    document_transition: DocumentTransitionWasm,
}

#[wasm_bindgen]
impl InvalidDocumentActionError {
    #[wasm_bindgen(constructor)]
    pub fn new(document_transition: DocumentTransitionWasm) -> Self {
        Self {
            document_transition,
        }
    }

    #[wasm_bindgen(js_name=getDocumentTransition)]
    pub fn get_document_transition(&self) -> DocumentTransitionWasm {
        self.document_transition.clone()
    }
}
