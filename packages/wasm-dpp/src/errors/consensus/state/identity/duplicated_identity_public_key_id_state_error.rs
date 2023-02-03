use dpp::identity::KeyID;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyIdStateError)]
pub struct DuplicatedIdentityPublicKeyIdStateErrorWasm {
    duplicated_ids: Vec<KeyID>,
    code: u32,
}

#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyIdStateError)]
impl DuplicatedIdentityPublicKeyIdStateErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedIds)]
    pub fn duplicated_ids(&self) -> js_sys::Array {
        // TODO: key ids probably should be u32
        self.duplicated_ids
            .iter()
            .map(|id| JsValue::from(*id as u32))
            .collect()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl DuplicatedIdentityPublicKeyIdStateErrorWasm {
    pub fn new(duplicated_ids: Vec<KeyID>, code: u32) -> Self {
        Self {
            duplicated_ids,
            code,
        }
    }
}
