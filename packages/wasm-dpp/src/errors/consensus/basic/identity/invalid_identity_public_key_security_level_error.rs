use crate::errors::PublicKeyValidationErrorWasm;
use dpp::codes::ErrorWithCode;
use dpp::consensus::basic::identity::{InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError};
use dpp::errors::consensus::basic::JsonSchemaError;
use dpp::errors::consensus::ConsensusError as DPPConsensusError;
use wasm_bindgen::prelude::*;
use dpp::identity::{Purpose, SecurityLevel};

#[wasm_bindgen(js_name=InvalidIdentityPublicKeySecurityLevelError)]
pub struct InvalidIdentityPublicKeySecurityLevelErrorWasm {
    inner: InvalidIdentityPublicKeySecurityLevelError,
}

impl From<&InvalidIdentityPublicKeySecurityLevelError> for InvalidIdentityPublicKeySecurityLevelErrorWasm {
    fn from(e: &InvalidIdentityPublicKeySecurityLevelError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityPublicKeySecurityLevelError)]
impl InvalidIdentityPublicKeySecurityLevelErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyId)]
    pub fn public_key_id(&self) -> u32 {
        // TODO: think about what to do with that conversion
        self.inner.public_key_id() as u32
    }

    #[wasm_bindgen(js_name=getPurpose)]
    pub fn purpose(&self) -> u8 {
        self.inner.purpose() as u8
    }

    #[wasm_bindgen(js_name=getSecurityLevel)]
    pub fn security_level(&self) -> u8 {
        self.inner.security_level() as u8
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        DPPConsensusError::from(self.inner.clone()).get_code()
    }
}
