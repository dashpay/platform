use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IncompatibleRe2PatternError)]
pub struct IncompatibleRe2PatternErrorWasm {
    pattern: String,
    path: String,
    message: String,
    code: u32,
}

impl IncompatibleRe2PatternErrorWasm {
    pub fn new(pattern: String, path: String, message: String, code: u32) -> Self {
        IncompatibleRe2PatternErrorWasm {
            pattern,
            path,
            message,
            code,
        }
    }
}

#[wasm_bindgen(js_class=IncompatibleRe2PatternError)]
impl IncompatibleRe2PatternErrorWasm {
    #[wasm_bindgen(js_name=getPattern)]
    pub fn get_pattern(&self) -> String {
        self.pattern.clone()
    }

    #[wasm_bindgen(js_name=getPath)]
    pub fn get_path(&self) -> String {
        self.path.clone()
    }

    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        self.message.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
