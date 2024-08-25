use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::identity::duplicated_identity_public_key_state_error::DuplicatedIdentityPublicKeyStateError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyStateError)]
pub struct DuplicatedIdentityPublicKeyStateErrorWasm {
    inner: DuplicatedIdentityPublicKeyStateError,
}

impl From<&DuplicatedIdentityPublicKeyStateError> for DuplicatedIdentityPublicKeyStateErrorWasm {
    fn from(e: &DuplicatedIdentityPublicKeyStateError) -> Self {
        Self { inner: e.clone() }
    }
}
#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyStateError)]
impl DuplicatedIdentityPublicKeyStateErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedPublicKeysIds)]
    pub fn duplicated_public_keys_ids(&self) -> js_sys::Array {
        // TODO: key ids probably should be u32
        self.inner
            .duplicated_public_key_ids()
            .iter()
            .map(|id| JsValue::from(*id))
            .collect()
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
