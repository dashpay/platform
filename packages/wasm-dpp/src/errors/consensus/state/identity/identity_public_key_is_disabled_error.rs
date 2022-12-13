use dpp::identity::KeyID;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityPublicKeyIsDisabledError)]
pub struct IdentityPublicKeyIsDisabledErrorWasm {
    public_key_index: KeyID,
    code: u32,
}

#[wasm_bindgen(js_class=IdentityPublicKeyIsDisabledError)]
impl IdentityPublicKeyIsDisabledErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyIndex)]
    pub fn public_key_index(&self) -> u32 {
        // TODO: make key ids u32?
        self.public_key_index as u32
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl IdentityPublicKeyIsDisabledErrorWasm {
    pub fn new(public_key_index: KeyID, code: u32) -> Self {
        Self { public_key_index, code }
    }
}
