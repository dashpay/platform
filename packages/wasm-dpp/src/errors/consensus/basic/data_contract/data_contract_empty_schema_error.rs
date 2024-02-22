use crate::buffer::Buffer;
use dpp::consensus::basic::data_contract::DataContractEmptySchemaError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractEmptySchemaError)]
pub struct DataContractEmptySchemaErrorWasm {
    inner: DataContractEmptySchemaError,
}

impl From<&DataContractEmptySchemaError> for DataContractEmptySchemaErrorWasm {
    fn from(e: &DataContractEmptySchemaError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractEmptySchemaError)]
impl DataContractEmptySchemaErrorWasm {
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
