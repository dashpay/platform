use dpp::consensus::basic::identity::DuplicatedIdentityPublicKeyBasicError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyError)]
pub struct DuplicatedIdentityPublicKeyErrorWasm {
    inner: DuplicatedIdentityPublicKeyBasicError,
}

impl From<&DuplicatedIdentityPublicKeyBasicError> for DuplicatedIdentityPublicKeyErrorWasm {
    fn from(e: &DuplicatedIdentityPublicKeyBasicError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyError)]
impl DuplicatedIdentityPublicKeyErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedPublicKeysIds)]
    pub fn duplicated_public_keys_ids(&self) -> js_sys::Array {
        // TODO: key ids probably should be u32
        self.inner
            .duplicated_public_keys_ids()
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
