use crate::buffer::Buffer;
use dpp::prelude::Identifier;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDocumentTransitionIdError)]
pub struct InvalidDocumentTransitionIdErrorWasm {
    expected_id: Identifier,
    invalid_id: Identifier,
    code: u32,
}

impl InvalidDocumentTransitionIdErrorWasm {
    pub fn new(expected_id: Identifier, invalid_id: Identifier, code: u32) -> Self {
        InvalidDocumentTransitionIdErrorWasm {
            expected_id,
            invalid_id,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidDocumentTransitionIdError)]
impl InvalidDocumentTransitionIdErrorWasm {
    #[wasm_bindgen(js_name=getExpectedId)]
    pub fn get_expected_id(&self) -> Buffer {
        Buffer::from_bytes(self.expected_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getInvalidId)]
    pub fn get_invalid_id(&self) -> Buffer {
        Buffer::from_bytes(self.invalid_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
