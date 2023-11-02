use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid document: {:?}", errors)]
pub struct InvalidDocumentError {
    // the point is how we hold all there different types in  the Vector
    errors: Vec<JsValue>,
    raw_document: JsValue,
}

#[wasm_bindgen(js_class=InvalidDocumentError)]
impl InvalidDocumentError {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_document: JsValue, errors: Vec<JsValue>) -> InvalidDocumentError {
        Self {
            raw_document,
            errors,
        }
    }

    #[wasm_bindgen(js_name=getErrors)]
    pub fn get_errors(&self) -> Vec<JsValue> {
        self.errors.clone()
    }

    #[wasm_bindgen(js_name=getRawDocument)]
    pub fn get_document(&self) -> JsValue {
        self.raw_document.clone()
    }

    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        self.to_string()
    }
}
