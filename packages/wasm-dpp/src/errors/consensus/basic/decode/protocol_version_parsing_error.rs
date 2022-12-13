use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ProtocolVersionParsingError)]
pub struct ProtocolVersionParsingErrorWasm {
    parsing_error: JsError,
    code: u32,
}

impl ProtocolVersionParsingErrorWasm {
    pub fn new(parsing_error: JsError, code: u32) -> Self {
        ProtocolVersionParsingErrorWasm {
            parsing_error,
            code,
        }
    }
}

#[wasm_bindgen(js_class=ProtocolVersionParsingError)]
impl ProtocolVersionParsingErrorWasm {
    #[wasm_bindgen(js_name=getParsingError)]
    pub fn get_parsing_error(&self) -> JsError {
        self.parsing_error.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
