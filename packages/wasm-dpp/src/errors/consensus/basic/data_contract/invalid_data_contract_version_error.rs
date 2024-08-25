use dpp::consensus::basic::data_contract::InvalidDataContractVersionError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDataContractVersionError)]
pub struct InvalidDataContractVersionErrorWasm {
    inner: InvalidDataContractVersionError,
}

impl From<&InvalidDataContractVersionError> for InvalidDataContractVersionErrorWasm {
    fn from(e: &InvalidDataContractVersionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidDataContractVersionError)]
impl InvalidDataContractVersionErrorWasm {
    #[wasm_bindgen(js_name=getExpectedVersion)]
    pub fn get_expected_version(&self) -> u32 {
        self.inner.expected_version()
    }

    #[wasm_bindgen(js_name=getVersion)]
    pub fn get_version(&self) -> u32 {
        self.inner.version()
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
