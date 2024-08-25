use dpp::consensus::basic::identity::InvalidIdentityPublicKeySecurityLevelError;
use dpp::errors::consensus::codes::ErrorWithCode;

use dpp::errors::consensus::ConsensusError as DPPConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityPublicKeySecurityLevelError)]
pub struct InvalidIdentityPublicKeySecurityLevelErrorWasm {
    inner: InvalidIdentityPublicKeySecurityLevelError,
}

impl From<&InvalidIdentityPublicKeySecurityLevelError>
    for InvalidIdentityPublicKeySecurityLevelErrorWasm
{
    fn from(e: &InvalidIdentityPublicKeySecurityLevelError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityPublicKeySecurityLevelError)]
impl InvalidIdentityPublicKeySecurityLevelErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyId)]
    pub fn public_key_id(&self) -> u32 {
        // TODO: think about what to do with that conversion
        self.inner.public_key_id()
    }

    #[wasm_bindgen(js_name=getPublicKeyPurpose)]
    pub fn purpose(&self) -> u8 {
        self.inner.purpose() as u8
    }

    #[wasm_bindgen(js_name=getPublicKeySecurityLevel)]
    pub fn security_level(&self) -> u8 {
        self.inner.security_level() as u8
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        DPPConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
