use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("No documents were supplied to state transition")]
pub struct NotDocumentsSuppliedError {}

#[wasm_bindgen]
impl NotDocumentsSuppliedError {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        NotDocumentsSuppliedError {}
    }
}
