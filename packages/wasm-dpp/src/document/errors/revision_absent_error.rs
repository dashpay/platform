use crate::document::extended_document::ExtendedDocumentWasm;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("The revision was absent, but was needed")]
pub struct RevisionAbsentError {
    extended_document: ExtendedDocumentWasm,
}

#[wasm_bindgen]
impl RevisionAbsentError {
    #[wasm_bindgen(constructor)]
    pub fn new(extended_document: ExtendedDocumentWasm) -> RevisionAbsentError {
        Self { extended_document }
    }

    #[wasm_bindgen(js_name=getDocument)]
    pub fn get_document_transition(&self) -> ExtendedDocumentWasm {
        self.extended_document.clone()
    }
}
