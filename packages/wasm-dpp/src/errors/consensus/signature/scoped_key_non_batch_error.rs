use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::ScopedKeyNonBatchError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ScopedKeyNonBatchError)]
pub struct ScopedKeyNonBatchErrorWasm {
    inner: ScopedKeyNonBatchError,
}

impl From<&ScopedKeyNonBatchError> for ScopedKeyNonBatchErrorWasm {
    fn from(e: &ScopedKeyNonBatchError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=ScopedKeyNonBatchError)]
impl ScopedKeyNonBatchErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
