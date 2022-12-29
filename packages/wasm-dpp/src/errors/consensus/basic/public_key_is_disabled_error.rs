use dpp::identity::KeyID;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=PublicKeyIsDisabledError)]
pub struct PublicKeyIsDisabledErrorWasm {
    public_key_id: KeyID,
    code: u32,
}

impl PublicKeyIsDisabledErrorWasm {
    pub fn new(public_key_id: KeyID, code: u32) -> Self {
        PublicKeyIsDisabledErrorWasm {
            public_key_id,
            code,
        }
    }
}

#[wasm_bindgen(js_class=PublicKeyIsDisabledError)]
impl PublicKeyIsDisabledErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyId)]
    pub fn get_public_key_id(&self) -> u64 {
        self.public_key_id
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
