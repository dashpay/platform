use crate::DocumentInStateTransitionWasm;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document Initial revision '{}'", document.get_revision())]
pub struct InvalidInitialRevisionError {
    document: DocumentInStateTransitionWasm,
}

#[wasm_bindgen]
impl InvalidInitialRevisionError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: DocumentInStateTransitionWasm) -> InvalidInitialRevisionError {
        Self { document }
    }

    #[wasm_bindgen(js_name=getDocument)]
    pub fn get_document_transition(&self) -> DocumentInStateTransitionWasm {
        self.document.clone()
    }
}
