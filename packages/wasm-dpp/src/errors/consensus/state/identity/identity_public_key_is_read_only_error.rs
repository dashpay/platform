use crate::buffer::Buffer;
use dpp::identifier::Identifier;
use wasm_bindgen::prelude::*;
use dpp::identity::KeyID;

#[wasm_bindgen(js_name=IdentityPublicKeyIsReadOnlyError)]
pub struct IdentityPublicKeyIsReadOnlyErrorWasm {
    key_id: KeyID,
    code: u32,
}

#[wasm_bindgen(js_class=IdentityPublicKeyIsReadOnlyError)]
impl IdentityPublicKeyIsReadOnlyErrorWasm {
    #[wasm_bindgen(js_name=getKeyId)]
    pub fn key_id(&self) -> u32 {
        // TODO: make key ids u32?
        self.key_id as u32
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl IdentityPublicKeyIsReadOnlyErrorWasm {
    pub fn new(
        key_id: KeyID,
        code: u32
    ) -> Self {
        Self {
            key_id,
            code,
        }
    }
}
