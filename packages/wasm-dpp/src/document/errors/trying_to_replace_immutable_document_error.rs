use crate::document::DocumentWasm;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Trying to update an immutable document")]
pub struct TryingToReplaceImmutableDocumentError {
    document: DocumentWasm,
}

#[wasm_bindgen]
impl TryingToReplaceImmutableDocumentError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: DocumentWasm) -> Self {
        TryingToReplaceImmutableDocumentError { document }
    }
}
