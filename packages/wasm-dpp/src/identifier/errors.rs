use thiserror::Error;
use wasm_bindgen::prelude::*;

#[derive(Error, Debug)]
#[wasm_bindgen(js_name=IdentifierError)]
#[error("{message}")]
pub struct IdentifierErrorWasm {
    message: String,
}

impl IdentifierErrorWasm {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[wasm_bindgen(js_class=IdentifierError)]
impl IdentifierErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_message(&self) -> String {
        self.message.clone()
    }

    #[wasm_bindgen(js_name=toString)]
    pub fn print(&self) -> String {
        format!("IdentifierError: {0}", { &self.message })
    }
}
