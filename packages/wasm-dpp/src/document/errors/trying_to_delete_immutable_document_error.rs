use crate::document::DocumentWasm;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Trying to delete an immutable document")]
pub struct TryingToDeleteImmutableDocumentError {
    document: DocumentWasm,
}

#[wasm_bindgen]
impl TryingToDeleteImmutableDocumentError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: DocumentWasm) -> Self {
        TryingToDeleteImmutableDocumentError { document }
    }
}
