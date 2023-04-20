use dpp::identity::KeyID;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingIdentityPublicKeyIdsError)]
pub struct MissingIdentityPublicKeyIdsErrorWasm {
    ids: Vec<KeyID>,
}

#[wasm_bindgen(js_class=MissingIdentityPublicKeyIdsError)]
impl MissingIdentityPublicKeyIdsErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedIds)]
    pub fn duplicated_ids(&self) -> js_sys::Array {
        // TODO: key ids probably should be u32
        self.ids.iter().map(|id| JsValue::from(*id)).collect()
    }
}

impl MissingIdentityPublicKeyIdsErrorWasm {
    pub fn new(ids: Vec<KeyID>) -> Self {
        Self { ids }
    }
}
