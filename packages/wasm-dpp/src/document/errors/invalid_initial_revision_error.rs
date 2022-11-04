use thiserror::Error;

use crate::DocumentWasm;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document Initial revision '{}'", document.get_revision())]
pub struct InvalidInitialRevisionError {
    document: DocumentWasm,
}

#[wasm_bindgen]
impl InvalidInitialRevisionError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: DocumentWasm) -> InvalidInitialRevisionError {
        Self { document }
    }

    #[wasm_bindgen(js_name=getDocumentTransition)]
    pub fn get_document_transition(&self) -> DocumentWasm {
        self.document.clone()
    }
}
