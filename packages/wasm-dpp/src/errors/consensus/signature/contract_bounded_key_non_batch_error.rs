use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::ContractBoundedKeyNonBatchError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ContractBoundedKeyNonBatchError)]
pub struct ContractBoundedKeyNonBatchErrorWasm {
    inner: ContractBoundedKeyNonBatchError,
}

impl From<&ContractBoundedKeyNonBatchError> for ContractBoundedKeyNonBatchErrorWasm {
    fn from(e: &ContractBoundedKeyNonBatchError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=ContractBoundedKeyNonBatchError)]
impl ContractBoundedKeyNonBatchErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
