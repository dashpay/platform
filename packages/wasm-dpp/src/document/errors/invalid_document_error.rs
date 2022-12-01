use thiserror::Error;

use crate::DocumentWasm;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid document: {:?}", errors)]
pub struct InvalidDocumentError {
    // the point is how we hold all there different types in  the Vector
    errors: Vec<JsValue>,
    document: DocumentWasm,
}

#[wasm_bindgen]
impl InvalidDocumentError {
    #[wasm_bindgen(constructor)]
    pub fn new(document: DocumentWasm, errors: Vec<JsValue>) -> InvalidDocumentError {
        Self { document, errors }
    }

    #[wasm_bindgen(js_name=getErrors)]
    pub fn get_errors(&self) -> Vec<JsValue> {
        self.errors.clone()
    }

    #[wasm_bindgen(js_name=getDocument)]
    pub fn get_document(&self) -> DocumentWasm {
        self.document.clone()
    }
}
