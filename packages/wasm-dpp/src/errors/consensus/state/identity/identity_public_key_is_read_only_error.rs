use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::identity::identity_public_key_is_read_only_error::IdentityPublicKeyIsReadOnlyError;
use dpp::consensus::ConsensusError;
use dpp::identity::KeyID;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityPublicKeyIsReadOnlyError)]
pub struct IdentityPublicKeyIsReadOnlyErrorWasm {
    inner: IdentityPublicKeyIsReadOnlyError,
}

impl From<&IdentityPublicKeyIsReadOnlyError> for IdentityPublicKeyIsReadOnlyErrorWasm {
    fn from(e: &IdentityPublicKeyIsReadOnlyError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityPublicKeyIsReadOnlyError)]
impl IdentityPublicKeyIsReadOnlyErrorWasm {
    #[wasm_bindgen(js_name=getKeyId)]
    pub fn key_id(&self) -> KeyID {
        self.inner.public_key_index()
    }

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
