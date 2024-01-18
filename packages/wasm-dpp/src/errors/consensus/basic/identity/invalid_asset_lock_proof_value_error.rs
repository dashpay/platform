use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use dpp::consensus::state::identity::invalid_asset_lock_proof_value::InvalidAssetLockProofValueError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidAssetLockProofValueError)]
pub struct InvalidAssetLockProofValueErrorWasm {
    inner: InvalidAssetLockProofValueError,
}

impl From<&InvalidAssetLockProofValueError> for InvalidAssetLockProofValueErrorWasm {
    fn from(e: &InvalidAssetLockProofValueError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidAssetLockProofValueError)]
impl InvalidAssetLockProofValueErrorWasm {
    #[wasm_bindgen(js_name=getValue)]
    pub fn value(&self) -> u64 {
        self.inner.value()
    }

    #[wasm_bindgen(js_name=getMinValue)]
    pub fn min_value(&self) -> u64 {
        self.inner.min_value()
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
