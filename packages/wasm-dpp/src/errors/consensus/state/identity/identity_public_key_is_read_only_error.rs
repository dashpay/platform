use dpp::identity::KeyID;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityPublicKeyIsReadOnlyError)]
pub struct IdentityPublicKeyIsReadOnlyErrorWasm {
    key_id: KeyID,
    code: u32,
}

#[wasm_bindgen(js_class=IdentityPublicKeyIsReadOnlyError)]
impl IdentityPublicKeyIsReadOnlyErrorWasm {
    #[wasm_bindgen(js_name=getKeyId)]
    pub fn key_id(&self) -> KeyID {
        self.key_id
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }

    #[wasm_bindgen(js_name=getPublicKeyIndex)]
    pub fn public_key_index(&self) -> KeyID {
        self.key_id
    }
}

impl IdentityPublicKeyIsReadOnlyErrorWasm {
    pub fn new(key_id: KeyID, code: u32) -> Self {
        Self { key_id, code }
    }
}
