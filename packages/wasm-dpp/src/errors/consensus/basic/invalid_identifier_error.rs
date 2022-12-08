
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentifierError)]
pub struct InvalidIdentifierErrorWasm {
    identifier_name: String,
    error: String,
    code: u32,
}

impl InvalidIdentifierErrorWasm {
    pub fn new(identifier_name: String, error: String, code: u32) -> Self {
        InvalidIdentifierErrorWasm {
            identifier_name,
            error,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidIdentifierError)]
impl InvalidIdentifierErrorWasm {
    #[wasm_bindgen(js_name=getIdentifierName)]
    pub fn get_identifier_name(&self) -> String {
        self.identifier_name.clone()
    }

    #[wasm_bindgen(js_name=getError)]
    pub fn get_error(&self) -> String {
        self.error.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
