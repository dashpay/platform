use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::ContractBoundedKeyOutOfBoundsError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ContractBoundedKeyOutOfBoundsError)]
pub struct ContractBoundedKeyOutOfBoundsErrorWasm {
    inner: ContractBoundedKeyOutOfBoundsError,
}

impl From<&ContractBoundedKeyOutOfBoundsError> for ContractBoundedKeyOutOfBoundsErrorWasm {
    fn from(e: &ContractBoundedKeyOutOfBoundsError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=ContractBoundedKeyOutOfBoundsError)]
impl ContractBoundedKeyOutOfBoundsErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
