use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("No documents were supplied to state transition")]
pub struct NoDocumentsSuppliedError {}

#[wasm_bindgen]
impl NoDocumentsSuppliedError {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        NoDocumentsSuppliedError {}
    }
}

impl Default for NoDocumentsSuppliedError {
    fn default() -> Self {
        Self::new()
    }
}
