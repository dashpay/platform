use crate::buffer::Buffer;
use dpp::prelude::Identifier;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDocumentTransitionActionError)]
pub struct InvalidDocumentTransitionActionErrorWasm {
    action: String,
    code: u32,
}

impl InvalidDocumentTransitionActionErrorWasm {
    pub fn new(action: String, code: u32) -> Self {
        InvalidDocumentTransitionActionErrorWasm { action, code }
    }
}

#[wasm_bindgen(js_class=InvalidDocumentTransitionActionError)]
impl InvalidDocumentTransitionActionErrorWasm {
    #[wasm_bindgen(js_name=getAction)]
    pub fn get_action(&self) -> String {
        self.action.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
