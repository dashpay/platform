use dpp::identity::Purpose;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=WrongPublicKeyPurposeError)]
pub struct WrongPublicKeyPurposeErrorWasm {
    public_key_purpose: Purpose,
    key_purpose_requirement: Purpose,
    code: u32,
}

impl WrongPublicKeyPurposeErrorWasm {
    pub fn new(public_key_purpose: Purpose, key_purpose_requirement: Purpose, code: u32) -> Self {
        WrongPublicKeyPurposeErrorWasm {
            public_key_purpose,
            key_purpose_requirement,
            code,
        }
    }
}

#[wasm_bindgen(js_class=WrongPublicKeyPurposeError)]
impl WrongPublicKeyPurposeErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyPurpose)]
    pub fn get_public_key_purpose(&self) -> u8 {
        self.public_key_purpose as u8
    }

    #[wasm_bindgen(js_name=getKeyPurposeRequirement)]
    pub fn get_key_purpose_requirement(&self) -> u8 {
        self.key_purpose_requirement as u8
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
