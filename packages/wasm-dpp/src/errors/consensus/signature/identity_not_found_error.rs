use wasm_bindgen::prelude::*;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::consensus::signature::IdentityNotFoundError;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=IdentityNotFoundError)]
pub struct IdentityNotFoundErrorWasm {
  inner: IdentityNotFoundError,
}

impl From<&IdentityNotFoundError> for IdentityNotFoundErrorWasm {
  fn from(e: &IdentityNotFoundError) -> Self {
    Self { inner: e.clone() }
  }
}

#[wasm_bindgen(js_class=IdentityNotFoundError)]
impl IdentityNotFoundErrorWasm {
    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn get_identity_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.identity_id().as_bytes())
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
