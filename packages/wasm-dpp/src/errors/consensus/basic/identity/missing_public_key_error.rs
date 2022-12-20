use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingPublicKeyError)]
pub struct MissingPublicKeyErrorWasm {
    public_key_id: u64,
    code: u32,
}

impl MissingPublicKeyErrorWasm {
    pub fn new(public_key_id: u64, code: u32) -> Self {
        MissingPublicKeyErrorWasm {
            public_key_id,
            code,
        }
    }
}

#[wasm_bindgen(js_class=MissingPublicKeyError)]
impl MissingPublicKeyErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyId)]
    pub fn get_public_key_id(&self) -> u64 {
        self.public_key_id
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
