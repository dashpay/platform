use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::SignatureShouldNotBePresentError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=SignatureShouldNotBePresentError)]
pub struct SignatureShouldNotBePresentErrorWasm {
    inner: SignatureShouldNotBePresentError,
}

impl From<&SignatureShouldNotBePresentError> for SignatureShouldNotBePresentErrorWasm {
    fn from(e: &SignatureShouldNotBePresentError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=SignatureShouldNotBePresentError)]
impl SignatureShouldNotBePresentErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
