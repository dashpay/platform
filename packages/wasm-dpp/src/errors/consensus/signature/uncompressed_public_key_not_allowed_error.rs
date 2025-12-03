use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::UncompressedPublicKeyNotAllowedError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=UncompressedPublicKeyNotAllowedError)]
pub struct UncompressedPublicKeyNotAllowedErrorWasm {
    inner: UncompressedPublicKeyNotAllowedError,
}

impl From<&UncompressedPublicKeyNotAllowedError> for UncompressedPublicKeyNotAllowedErrorWasm {
    fn from(e: &UncompressedPublicKeyNotAllowedError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=UncompressedPublicKeyNotAllowedError)]
impl UncompressedPublicKeyNotAllowedErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name=getPublicKeySize)]
    pub fn get_public_key_size(&self) -> usize {
        self.inner.public_key_size()
    }
}
