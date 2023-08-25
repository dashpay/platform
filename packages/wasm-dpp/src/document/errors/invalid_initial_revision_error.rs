use crate::document::extended_document::ExtendedDocumentWasm;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document Initial revision '{}'", document.get_revision().unwrap_or_default())]
pub struct InvalidInitialRevisionError {
    document: ExtendedDocumentWasm,
}

#[wasm_bindgen]
impl InvalidInitialRevisionError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: ExtendedDocumentWasm) -> InvalidInitialRevisionError {
        Self { document }
    }

    #[wasm_bindgen(js_name=getDocument)]
    pub fn get_document_transition(&self) -> ExtendedDocumentWasm {
        self.document.clone()
    }
}
