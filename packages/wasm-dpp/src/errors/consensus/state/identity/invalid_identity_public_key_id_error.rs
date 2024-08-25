use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::identity::invalid_identity_public_key_id_error::InvalidIdentityPublicKeyIdError;
use dpp::consensus::ConsensusError;
use dpp::identity::KeyID;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityPublicKeyIdError)]
pub struct InvalidIdentityPublicKeyIdErrorWasm {
    inner: InvalidIdentityPublicKeyIdError,
}

impl From<&InvalidIdentityPublicKeyIdError> for InvalidIdentityPublicKeyIdErrorWasm {
    fn from(e: &InvalidIdentityPublicKeyIdError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityPublicKeyIdError)]
impl InvalidIdentityPublicKeyIdErrorWasm {
    #[wasm_bindgen(js_name=getId)]
    pub fn id(&self) -> KeyID {
        self.inner.id()
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
