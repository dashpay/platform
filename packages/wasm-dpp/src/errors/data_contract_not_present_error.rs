use dpp::prelude::Identifier;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=DataContractNotPresentNotConsensusError)]
pub struct DataContractNotPresentNotConsensusErrorWasm {
    data_contract_id: Identifier,
}

impl DataContractNotPresentNotConsensusErrorWasm {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self { data_contract_id }
    }
}

#[wasm_bindgen(js_class=DataContractNotPresentNotConsensusError)]
impl DataContractNotPresentNotConsensusErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.data_contract_id.as_bytes())
    }
}
