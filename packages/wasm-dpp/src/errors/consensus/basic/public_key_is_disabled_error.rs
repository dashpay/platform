use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::PublicKeyIsDisabledError;
use dpp::consensus::ConsensusError;
use dpp::identity::KeyID;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=PublicKeyIsDisabledError)]
pub struct PublicKeyIsDisabledErrorWasm {
    inner: PublicKeyIsDisabledError,
}

impl From<&PublicKeyIsDisabledError> for PublicKeyIsDisabledErrorWasm {
    fn from(e: &PublicKeyIsDisabledError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=PublicKeyIsDisabledError)]
impl PublicKeyIsDisabledErrorWasm {
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
