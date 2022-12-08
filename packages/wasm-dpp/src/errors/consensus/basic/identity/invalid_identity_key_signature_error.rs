
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityKeySignatureError)]
pub struct InvalidIdentityKeySignatureErrorWasm {
    public_key_id: u32,
    code: u32,
}

impl InvalidIdentityKeySignatureErrorWasm {
    pub fn new(public_key_id: u32, code: u32) -> Self {
        InvalidIdentityKeySignatureErrorWasm {
            public_key_id,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityKeySignatureError)]
impl InvalidIdentityKeySignatureErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyId)]
    pub fn get_public_key_id(&self) -> u32 {
        self.public_key_id
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
