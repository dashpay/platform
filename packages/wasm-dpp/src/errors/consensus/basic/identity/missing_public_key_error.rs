use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::MissingPublicKeyError;
use dpp::consensus::ConsensusError;
use dpp::identity::KeyID;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingPublicKeyError)]
pub struct MissingPublicKeyErrorWasm {
    inner: MissingPublicKeyError,
}

impl From<&MissingPublicKeyError> for MissingPublicKeyErrorWasm {
    fn from(e: &MissingPublicKeyError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=MissingPublicKeyError)]
impl MissingPublicKeyErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyId)]
    pub fn get_public_key_id(&self) -> KeyID {
        self.inner.public_key_id()
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
