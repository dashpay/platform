use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::InvalidSignaturePublicKeySecurityLevelError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidSignaturePublicKeySecurityLevelError)]
pub struct InvalidSignaturePublicKeySecurityLevelErrorWasm {
    inner: InvalidSignaturePublicKeySecurityLevelError,
}

impl From<&InvalidSignaturePublicKeySecurityLevelError>
    for InvalidSignaturePublicKeySecurityLevelErrorWasm
{
    fn from(e: &InvalidSignaturePublicKeySecurityLevelError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidSignaturePublicKeySecurityLevelError)]
impl InvalidSignaturePublicKeySecurityLevelErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeySecurityLevel)]
    pub fn get_public_key_security_level(&self) -> u8 {
        self.inner.public_key_security_level() as u8
    }

    #[wasm_bindgen(js_name=getKeySecurityLevelRequirement)]
    pub fn get_allowed_key_security_levels(&self) -> js_sys::Array {
        let array = js_sys::Array::new();
        for security_level in self.inner.allowed_key_security_levels() {
            array.push(&JsValue::from(security_level as u32));
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
