use dpp::consensus::basic::identity::InvalidAuthenticationScopeError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidAuthenticationScopeError)]
pub struct InvalidAuthenticationScopeErrorWasm {
    inner: InvalidAuthenticationScopeError,
}

impl From<&InvalidAuthenticationScopeError> for InvalidAuthenticationScopeErrorWasm {
    fn from(e: &InvalidAuthenticationScopeError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidAuthenticationScopeError)]
impl InvalidAuthenticationScopeErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
