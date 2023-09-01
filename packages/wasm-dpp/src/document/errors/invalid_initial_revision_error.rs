use crate::document::DocumentWasm;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document Initial revision '{}'", document.get_revision().unwrap_or_default())]
pub struct InvalidInitialRevisionError {
    document: DocumentWasm,
}

#[wasm_bindgen]
impl InvalidInitialRevisionError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: DocumentWasm) -> InvalidInitialRevisionError {
        Self { document }
    }

    #[wasm_bindgen(js_name=getDocument)]
    pub fn get_document_transition(&self) -> DocumentWasm {
        self.document.clone()
    }
}
