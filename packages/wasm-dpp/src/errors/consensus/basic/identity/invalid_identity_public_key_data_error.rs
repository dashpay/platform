use dpp::consensus::basic::identity::InvalidIdentityPublicKeyDataError;
use dpp::errors::consensus::codes::ErrorWithCode;

use dpp::errors::consensus::ConsensusError as DPPConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityPublicKeyDataError)]
pub struct InvalidIdentityPublicKeyDataErrorWasm {
    inner: InvalidIdentityPublicKeyDataError,
}

impl From<&InvalidIdentityPublicKeyDataError> for InvalidIdentityPublicKeyDataErrorWasm {
    fn from(e: &InvalidIdentityPublicKeyDataError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityPublicKeyDataError)]
impl InvalidIdentityPublicKeyDataErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyId)]
    pub fn public_key_id(&self) -> u32 {
        // TODO: think about what to do with that conversion
        self.inner.public_key_id()
    }

    #[wasm_bindgen(js_name=getValidationError)]
    pub fn validation_error(&self) -> String {
        self.inner.validation_error().to_string()
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
