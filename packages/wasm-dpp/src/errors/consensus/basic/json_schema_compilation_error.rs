use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=JsonSchemaCompilationError)]
pub struct JsonSchemaCompilationErrorWasm {
    error: String,
    code: u32,
}

impl JsonSchemaCompilationErrorWasm {
    pub fn new(error: String, code: u32) -> Self {
        JsonSchemaCompilationErrorWasm { error, code }
    }
}

#[wasm_bindgen(js_class=JsonSchemaCompilationError)]
impl JsonSchemaCompilationErrorWasm {
    #[wasm_bindgen(js_name=getError)]
    pub fn get_error(&self) -> String {
        self.error
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
