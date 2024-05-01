use crate::document::DocumentWasm;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("The revision was absent, but was needed")]
pub struct RevisionAbsentError {
    document: DocumentWasm,
}

#[wasm_bindgen]
impl RevisionAbsentError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: DocumentWasm) -> RevisionAbsentError {
        Self { document }
    }

    #[wasm_bindgen(js_name=getDocument)]
    pub fn get_document_transition(&self) -> DocumentWasm {
        self.document.clone()
    }
}
