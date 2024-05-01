use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MaxIdentityPublicKeyLimitReachedError)]
pub struct MaxIdentityPublicKeyLimitReachedErrorWasm {
    // TODO we can't use usize otherwise it might be a big int in JS if count is too high
    inner: MaxIdentityPublicKeyLimitReachedError,
}

impl From<&MaxIdentityPublicKeyLimitReachedError> for MaxIdentityPublicKeyLimitReachedErrorWasm {
    fn from(e: &MaxIdentityPublicKeyLimitReachedError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=MaxIdentityPublicKeyLimitReachedError)]
impl MaxIdentityPublicKeyLimitReachedErrorWasm {
    #[wasm_bindgen(js_name=getMaxItems)]
    pub fn max_items(&self) -> usize {
        self.inner.max_items()
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
