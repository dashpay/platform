use thiserror::Error;

use crate::DocumentWasm;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Document no revision")]
pub struct DocumentNoRevisionError {
    document: DocumentWasm,
}

#[wasm_bindgen]
impl DocumentNoRevisionError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: DocumentWasm) -> DocumentNoRevisionError {
        Self { document }
    }

    #[wasm_bindgen(js_name=getDocument)]
    pub fn get_document_transition(&self) -> DocumentWasm {
        self.document.clone()
    }
}
