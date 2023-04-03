use wasm_bindgen::prelude::*;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::identity::duplicated_identity_public_key_id_state_error::DuplicatedIdentityPublicKeyIdStateError;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=DuplicatedIdentityPublicKeyIdStateError)]
pub struct DuplicatedIdentityPublicKeyIdStateErrorWasm {
  inner: DuplicatedIdentityPublicKeyIdStateError,
}

impl From<&DuplicatedIdentityPublicKeyIdStateError> for DuplicatedIdentityPublicKeyIdStateErrorWasm {
  fn from(e: &DuplicatedIdentityPublicKeyIdStateError) -> Self {
    Self { inner: e.clone() }
  }
}

#[wasm_bindgen(js_class=DuplicatedIdentityPublicKeyIdStateError)]
impl DuplicatedIdentityPublicKeyIdStateErrorWasm {
    #[wasm_bindgen(js_name=getDuplicatedIds)]
    pub fn duplicated_ids(&self) -> js_sys::Array {
        // TODO: key ids probably should be u32
        self.inner.duplicated_ids()
            .iter()
            .map(|id| JsValue::from(*id))
            .collect()
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
