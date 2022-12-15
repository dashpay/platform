use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidJsonSchemaRefError)]
pub struct InvalidJsonSchemaRefErrorWasm {
    ref_error: String,
    code: u32,
}

impl InvalidJsonSchemaRefErrorWasm {
    pub fn new(ref_error: String, code: u32) -> Self {
        InvalidJsonSchemaRefErrorWasm { ref_error, code }
    }
}

#[wasm_bindgen(js_class=InvalidJsonSchemaRefError)]
impl InvalidJsonSchemaRefErrorWasm {
    #[wasm_bindgen(js_name=getRefError)]
    pub fn get_ref_error(&self) -> String {
        self.ref_error.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
