use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=DataContractNotPresentError)]
pub struct DataContractNotPresentErrorWasm {
    inner: DataContractNotPresentError,
}

impl From<&DataContractNotPresentError> for DataContractNotPresentErrorWasm {
    fn from(e: &DataContractNotPresentError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractNotPresentError)]
impl DataContractNotPresentErrorWasm {
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
