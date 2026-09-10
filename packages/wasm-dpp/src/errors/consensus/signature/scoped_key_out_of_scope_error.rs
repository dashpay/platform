use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::ScopedKeyOutOfScopeError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ScopedKeyOutOfScopeError)]
pub struct ScopedKeyOutOfScopeErrorWasm {
    inner: ScopedKeyOutOfScopeError,
}

impl From<&ScopedKeyOutOfScopeError> for ScopedKeyOutOfScopeErrorWasm {
    fn from(e: &ScopedKeyOutOfScopeError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=ScopedKeyOutOfScopeError)]
impl ScopedKeyOutOfScopeErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
