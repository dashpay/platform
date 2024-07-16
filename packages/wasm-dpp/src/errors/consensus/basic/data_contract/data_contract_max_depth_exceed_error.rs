use dpp::consensus::basic::data_contract::data_contract_max_depth_exceed_error::DataContractMaxDepthExceedError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractMaxDepthExceedError)]
pub struct DataContractMaxDepthExceedErrorWasm {
    inner: DataContractMaxDepthExceedError,
}

impl From<&DataContractMaxDepthExceedError> for DataContractMaxDepthExceedErrorWasm {
    fn from(e: &DataContractMaxDepthExceedError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractMaxDepthError)]
impl DataContractMaxDepthExceedErrorWasm {
    #[wasm_bindgen(js_name=getMaxDepth)]
    pub fn get_max_depth(&self) -> usize {
        self.inner.max_depth()
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
