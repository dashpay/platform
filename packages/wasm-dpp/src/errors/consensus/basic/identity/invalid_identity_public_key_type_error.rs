use std::convert::TryFrom;

use crate::utils::WithJsError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::InvalidIdentityPublicKeyTypeError;
use dpp::consensus::ConsensusError;
use dpp::identity::KeyType;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIdentityPublicKeyTypeError)]
pub struct InvalidIdentityPublicKeyTypeErrorWasm {
    inner: InvalidIdentityPublicKeyTypeError,
}

impl From<&InvalidIdentityPublicKeyTypeError> for InvalidIdentityPublicKeyTypeErrorWasm {
    fn from(e: &InvalidIdentityPublicKeyTypeError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityPublicKeyTypeError)]
impl InvalidIdentityPublicKeyTypeErrorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(key_type: u8) -> Result<InvalidIdentityPublicKeyTypeErrorWasm, JsValue> {
        Ok(Self {
            inner: InvalidIdentityPublicKeyTypeError::new(
                KeyType::try_from(key_type).with_js_error()?,
            ),
        })
    }

    #[wasm_bindgen(js_name=getPublicKeyType)]
    pub fn get_public_key_type(&self) -> u8 {
        self.inner.public_key_type() as u8
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
