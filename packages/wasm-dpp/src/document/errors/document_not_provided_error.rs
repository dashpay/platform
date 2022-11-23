use thiserror::Error;

use crate::mocks::DocumentTransitionWasm;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Document was not provided for apply of state transition")]
pub struct DocumentNotProvidedError {
    document_transition: DocumentTransitionWasm,
}

#[wasm_bindgen]
impl DocumentNotProvidedError {
    #[wasm_bindgen(constructor)]
    pub fn new(document_transition: DocumentTransitionWasm) -> DocumentNotProvidedError {
        Self {
            document_transition,
        }
    }

    #[wasm_bindgen(js_name=getDocumentTransition)]
    pub fn get_document_transition(&self) -> DocumentTransitionWasm {
        self.document_transition.clone()
    }
}
