use dpp::identity::KeyType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityPublicKeyTypeError)]
pub struct InvalidIdentityPublicKeyTypeErrorWasm {
    public_key_type: KeyType,
    code: u32,
}

impl InvalidIdentityPublicKeyTypeErrorWasm {
    pub fn new(public_key_type: KeyType, code: u32) -> Self {
        InvalidIdentityPublicKeyTypeErrorWasm {
            public_key_type,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityPublicKeyTypeError)]
impl InvalidIdentityPublicKeyTypeErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyType)]
    pub fn get_public_key_type(&self) -> u8 {
        self.public_key_type as u8
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
