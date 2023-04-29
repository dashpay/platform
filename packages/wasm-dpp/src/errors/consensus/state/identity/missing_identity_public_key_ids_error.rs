use dpp::consensus::state::identity::missing_identity_public_key_ids_error::MissingIdentityPublicKeyIdsError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingIdentityPublicKeyIdsError)]
pub struct MissingIdentityPublicKeyIdsErrorWasm {
    inner: MissingIdentityPublicKeyIdsError,
}

impl From<&MissingIdentityPublicKeyIdsError> for MissingIdentityPublicKeyIdsErrorWasm {
    fn from(e: &MissingIdentityPublicKeyIdsError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=MissingIdentityPublicKeyIdsError)]
impl MissingIdentityPublicKeyIdsErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedIds)]
    pub fn duplicated_ids(&self) -> js_sys::Array {
        // TODO: key ids probably should be u32
        self.inner
            .ids()
            .iter()
            .map(|id| JsValue::from(*id))
            .collect()
    }
}
