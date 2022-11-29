use crate::buffer::Buffer;
use dpp::identifier::Identifier;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractAlreadyPresentError)]
pub struct DataContractAlreadyPresentErrorWasm {
    data_contract_id: Identifier,
    code: u32,
}

#[wasm_bindgen(js_class=DataContractAlreadyPresentError)]
impl DataContractAlreadyPresentErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.data_contract_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl DataContractAlreadyPresentErrorWasm {
    pub fn new(data_contract_id: Identifier, code: u32) -> Self {
        Self {
            data_contract_id,
            code,
        }
    }
}
