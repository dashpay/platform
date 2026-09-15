use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::ScopedKeyExpiredError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ScopedKeyExpiredError)]
pub struct ScopedKeyExpiredErrorWasm {
    inner: ScopedKeyExpiredError,
}

impl From<&ScopedKeyExpiredError> for ScopedKeyExpiredErrorWasm {
    fn from(e: &ScopedKeyExpiredError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=ScopedKeyExpiredError)]
impl ScopedKeyExpiredErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
