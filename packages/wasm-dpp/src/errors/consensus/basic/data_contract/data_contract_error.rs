use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use dpp::data_contract::errors::DataContractError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractError)]
pub struct DataContractErrorWasm {
    inner: DataContractError,
}

impl From<&DataContractError> for DataContractErrorWasm {
    fn from(e: &DataContractError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DataContractError)]
impl DataContractErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
