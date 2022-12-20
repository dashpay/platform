use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=SerializedObjectParsingError)]
pub struct SerializedObjectParsingErrorWasm {
    parsing_error: JsError,
    code: u32,
}

impl SerializedObjectParsingErrorWasm {
    pub fn new(parsing_error: JsError, code: u32) -> Self {
        SerializedObjectParsingErrorWasm {
            parsing_error,
            code,
        }
    }
}

#[wasm_bindgen(js_class=SerializedObjectParsingError)]
impl SerializedObjectParsingErrorWasm {
    #[wasm_bindgen(js_name=getParsingError)]
    pub fn get_parsing_error(&self) -> JsError {
        self.parsing_error.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
