use dpp::prelude::Identifier;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=DataContractNotPresentError)]
pub struct DataContractNotPresentErrorWasm {
    data_contract_id: Identifier,
    code: u32,
}

impl DataContractNotPresentErrorWasm {
    pub fn new(data_contract_id: Identifier, code: u32) -> Self {
        DataContractNotPresentErrorWasm {
            data_contract_id,
            code,
        }
    }
}

#[wasm_bindgen(js_class=DataContractNotPresentError)]
impl DataContractNotPresentErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.data_contract_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
