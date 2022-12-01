use dpp::identity::KeyID;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityPublicKeyIdError)]
pub struct InvalidIdentityPublicKeyIdErrorWasm {
    id: KeyID,
    code: u32,
}

#[wasm_bindgen(js_class=InvalidIdentityPublicKeyIdError)]
impl InvalidIdentityPublicKeyIdErrorWasm {
    #[wasm_bindgen(js_name=getId)]
    pub fn id(&self) -> u32 {
        // TODO: make key ids u32?
        self.id as u32
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl InvalidIdentityPublicKeyIdErrorWasm {
    pub fn new(id: KeyID, code: u32) -> Self {
        Self { id, code }
    }
}
