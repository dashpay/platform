use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingDocumentTransitionActionError)]
pub struct MissingDocumentTransitionActionErrorWasm {
    code: u32,
}

impl MissingDocumentTransitionActionErrorWasm {
    pub fn new(code: u32) -> Self {
        MissingDocumentTransitionActionErrorWasm { code }
    }
}

#[wasm_bindgen(js_class=MissingDocumentTransitionActionError)]
impl MissingDocumentTransitionActionErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
