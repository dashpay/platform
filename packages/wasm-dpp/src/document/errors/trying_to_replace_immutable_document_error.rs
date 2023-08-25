use crate::document::extended_document::ExtendedDocumentWasm;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Trying to update an immutable document")]
pub struct TryingToReplaceImmutableDocumentError {
    extended_document: ExtendedDocumentWasm,
}

#[wasm_bindgen]
impl TryingToReplaceImmutableDocumentError {
    #[wasm_bindgen(constructor)]
    pub fn new(extended_document: ExtendedDocumentWasm) -> Self {
        TryingToReplaceImmutableDocumentError { extended_document }
    }
}
