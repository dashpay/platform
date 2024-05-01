use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::PublicKeySecurityLevelNotMetError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=PublicKeySecurityLevelNotMetError)]
pub struct PublicKeySecurityLevelNotMetErrorWasm {
    inner: PublicKeySecurityLevelNotMetError,
}

impl From<&PublicKeySecurityLevelNotMetError> for PublicKeySecurityLevelNotMetErrorWasm {
    fn from(e: &PublicKeySecurityLevelNotMetError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=PublicKeySecurityLevelNotMetError)]
impl PublicKeySecurityLevelNotMetErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeySecurityLevel)]
    pub fn get_public_key_security_level(&self) -> u8 {
        self.inner.public_key_security_level() as u8
    }

    #[wasm_bindgen(js_name=getKeySecurityLevelRequirement)]
    pub fn get_key_security_level_requirement(&self) -> u8 {
        self.inner.required_security_level() as u8
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
