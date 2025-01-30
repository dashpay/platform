use crate::document::DocumentWasm;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Trying to transfer an non transferable document")]
pub struct TryingToTransferNonTransferableDocumentError {
    document: DocumentWasm,
}

#[wasm_bindgen]
impl TryingToTransferNonTransferableDocumentError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: DocumentWasm) -> Self {
        TryingToTransferNonTransferableDocumentError { document }
    }
}
