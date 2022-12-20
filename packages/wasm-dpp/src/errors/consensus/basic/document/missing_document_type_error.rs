use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingDocumentTypeError)]
pub struct MissingDocumentTypeErrorWasm {
    code: u32,
}

impl MissingDocumentTypeErrorWasm {
    pub fn new(code: u32) -> Self {
        MissingDocumentTypeErrorWasm { code }
    }
}

#[wasm_bindgen(js_class=MissingDocumentTypeError)]
impl MissingDocumentTypeErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
