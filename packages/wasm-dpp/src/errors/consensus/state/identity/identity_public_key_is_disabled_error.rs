use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::identity::identity_public_key_is_disabled_error::IdentityPublicKeyIsDisabledError;
use dpp::consensus::ConsensusError;
use dpp::identity::KeyID;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityPublicKeyIsDisabledError)]
pub struct IdentityPublicKeyIsDisabledErrorWasm {
    inner: IdentityPublicKeyIsDisabledError,
}

impl From<&IdentityPublicKeyIsDisabledError> for IdentityPublicKeyIsDisabledErrorWasm {
    fn from(e: &IdentityPublicKeyIsDisabledError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityPublicKeyIsDisabledError)]
impl IdentityPublicKeyIsDisabledErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyIndex)]
    pub fn public_key_index(&self) -> KeyID {
        self.inner.public_key_index()
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
