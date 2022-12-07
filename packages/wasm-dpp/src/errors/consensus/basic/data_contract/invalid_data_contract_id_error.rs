use wasm_bindgen::prelude::*;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=InvalidDataContractIdErrorError)]
pub struct InvalidDataContractIdErrorWasm {
    expected_id: Vec<u8>,
    invalid_id: Vec<u8>,
    code: u32,
}

impl InvalidDataContractIdErrorWasm {
    pub fn new(expected_id: Vec<u8>, invalid_id: Vec<u8>, code: u32) -> Self {
        InvalidDataContractIdErrorWasm {
            expected_id,
            invalid_id,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidDataContractVersionError)]
impl InvalidDataContractIdErrorWasm {
    #[wasm_bindgen(js_name=getExpectedId)]
    pub fn get_expected_id(&self) -> Buffer {
        Buffer::from_bytes(&self.expected_id)
    }

    #[wasm_bindgen(js_name=getInvalidId)]
    pub fn get_invalid_id(&self) -> Buffer {
        Buffer::from_bytes(&self.invalid_id)
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
