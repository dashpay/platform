use wasm_bindgen::prelude::*;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::consensus::signature::InvalidSignaturePublicKeySecurityLevelError;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=InvalidSignaturePublicKeySecurityLevelError)]
pub struct InvalidSignaturePublicKeySecurityLevelErrorWasm {
  inner: InvalidSignaturePublicKeySecurityLevelError,
}

impl From<&InvalidSignaturePublicKeySecurityLevelError> for InvalidSignaturePublicKeySecurityLevelErrorWasm {
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

    #[wasm_bindgen(js_name=getRequiredKeySecurityLevel)]
    pub fn get_required_key_security_level(&self) -> u8 {
        self.inner.required_key_security_level() as u8
    }

  #[wasm_bindgen(js_name=getCode)]
  pub fn get_code(&self) -> u32 {
    ConsensusError::from(self.inner.clone()).code()
  }

  #[wasm_bindgen(js_name=serialize)]
  pub fn serialize(&self) -> Result<Buffer, JsError> {
    let bytes = ConsensusError::from(self.inner.clone())
      .serialize()
      .map_err(|e| JsError::from(e))?;

    Ok(Buffer::from_bytes(bytes.as_slice()))
  }
}
