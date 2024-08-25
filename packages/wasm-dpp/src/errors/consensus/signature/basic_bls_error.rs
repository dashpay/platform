use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::BasicBLSError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=BasicBLSError)]
pub struct BasicBLSErrorWasm {
    inner: BasicBLSError,
}

impl From<&BasicBLSError> for BasicBLSErrorWasm {
    fn from(e: &BasicBLSError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=BasicBLSError)]
impl BasicBLSErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
