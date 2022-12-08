use dpp::prelude::Identifier;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=IdentityNotFoundError)]
pub struct IdentityNotFoundErrorWasm {
    identity_id: Identifier,
    code: u32,
}

impl IdentityNotFoundErrorWasm {
    pub fn new(identity_id: Identifier, code: u32) -> Self {
        IdentityNotFoundErrorWasm { identity_id, code }
    }
}

#[wasm_bindgen(js_class=IdentityNotFoundError)]
impl IdentityNotFoundErrorWasm {
    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn get_identity_id(&self) -> Buffer {
        Buffer::from_bytes(self.identity_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
