use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingDocumentTransitionTypeError)]
pub struct MissingDocumentTransitionTypeErrorWasm {
    code: u32,
}

impl MissingDocumentTransitionTypeErrorWasm {
    pub fn new(code: u32) -> Self {
        MissingDocumentTransitionTypeErrorWasm { code }
    }
}

#[wasm_bindgen(js_class=MissingDocumentTypeError)]
impl MissingDocumentTransitionTypeErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
