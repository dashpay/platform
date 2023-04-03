use dpp::consensus::basic::identity::InvalidInstantAssetLockProofError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=InvalidInstantAssetLockProofError)]
pub struct InvalidInstantAssetLockProofErrorWasm {
    inner: InvalidInstantAssetLockProofError,
}

impl From<&InvalidInstantAssetLockProofError> for InvalidInstantAssetLockProofErrorWasm {
    fn from(e: &InvalidInstantAssetLockProofError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidInstantAssetLockProofError)]
impl InvalidInstantAssetLockProofErrorWasm {
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
