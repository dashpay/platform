use crate::buffer::Buffer;
use dpp::consensus::basic::data_contract::DocumentTypesAreMissingError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractEmptySchemaError)]
pub struct DocumentTypesAreMissingErrorWasm {
    inner: DocumentTypesAreMissingError,
}

impl From<&DocumentTypesAreMissingError> for DocumentTypesAreMissingErrorWasm {
    fn from(e: &DocumentTypesAreMissingError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractEmptySchemaError)]
impl DocumentTypesAreMissingErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.data_contract_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
