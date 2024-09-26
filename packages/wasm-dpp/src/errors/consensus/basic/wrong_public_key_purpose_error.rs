use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::WrongPublicKeyPurposeError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=WrongPublicKeyPurposeError)]
pub struct WrongPublicKeyPurposeErrorWasm {
    inner: WrongPublicKeyPurposeError,
}

impl From<&WrongPublicKeyPurposeError> for WrongPublicKeyPurposeErrorWasm {
    fn from(e: &WrongPublicKeyPurposeError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=WrongPublicKeyPurposeError)]
impl WrongPublicKeyPurposeErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyPurpose)]
    pub fn get_public_key_purpose(&self) -> u8 {
        self.inner.public_key_purpose() as u8
    }

    #[wasm_bindgen(js_name=getKeyPurposeRequirement)]
    pub fn get_allowed_key_purposes(&self) -> js_sys::Array {
        let array = js_sys::Array::new();
        for purpose in self.inner.allowed_key_purposes() {
            array.push(&JsValue::from(*purpose as u8));
        }
        array
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
