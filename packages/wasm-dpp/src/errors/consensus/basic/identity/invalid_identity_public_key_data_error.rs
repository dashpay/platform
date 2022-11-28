use crate::errors::PublicKeyValidationErrorWasm;
use dpp::codes::ErrorWithCode;
use dpp::consensus::basic::identity::InvalidIdentityPublicKeyDataError;
use dpp::errors::consensus::basic::JsonSchemaError;
use dpp::errors::consensus::ConsensusError as DPPConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityPublicKeyDataError)]
pub struct InvalidIdentityPublicKeyDataErrorWasm {
    inner: InvalidIdentityPublicKeyDataError,
}

impl From<InvalidIdentityPublicKeyDataError> for InvalidIdentityPublicKeyDataErrorWasm {
    fn from(e: InvalidIdentityPublicKeyDataError) -> Self {
        Self { inner: e }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityPublicKeyDataError)]
impl InvalidIdentityPublicKeyDataErrorWasm {
    #[wasm_bindgen(js_name=getPublicKeyId)]
    pub fn public_key_id(&self) -> u32 {
        // TODO: think about what to do with that conversion
        self.inner.public_key_id() as u32
    }

    #[wasm_bindgen(js_name=getValidationError)]
    pub fn validation_error(&self) -> Option<PublicKeyValidationErrorWasm> {
        if let Some(err) = self.inner.validation_error() {
            Some(PublicKeyValidationErrorWasm::from(err.clone()))
        } else {
            None
        }
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn code(&self) -> u32 {
        DPPConsensusError::from(self.inner.clone()).get_code()
    }
}
