use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::InvalidIdentityPublicKeyTypeError;
use dpp::consensus::ConsensusError;
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

    #[wasm_bindgen(js_name=serialize)]
    pub fn serialize(&self) -> Result<Buffer, JsError> {
        let bytes = ConsensusError::from(self.inner.clone())
            .serialize()
            .map_err(|e| JsError::from(e))?;

        Ok(Buffer::from_bytes(bytes.as_slice()))
    }
}
