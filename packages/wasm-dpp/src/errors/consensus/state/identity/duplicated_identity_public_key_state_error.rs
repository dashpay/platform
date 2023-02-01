use dpp::identity::KeyID;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyStateError)]
pub struct DuplicatedIdentityPublicKeyStateErrorWasm {
    duplicated_public_keys_ids: Vec<KeyID>,
    code: u32,
}

#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyStateError)]
impl DuplicatedIdentityPublicKeyStateErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedPublicKeysIds)]
    pub fn duplicated_public_keys_ids(&self) -> js_sys::Array {
        // TODO: key ids probably should be u32
        self.duplicated_public_keys_ids
            .iter()
            .map(|id| JsValue::from(*id as u32))
            .collect()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl DuplicatedIdentityPublicKeyStateErrorWasm {
    pub fn new(duplicated_public_keys_ids: Vec<KeyID>, code: u32) -> Self {
        Self {
            duplicated_public_keys_ids,
            code,
        }
    }
}
