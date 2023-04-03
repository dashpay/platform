use wasm_bindgen::prelude::*;
use dpp::consensus::basic::document::MissingDocumentTransitionActionError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=MissingDocumentTransitionActionError)]
pub struct MissingDocumentTransitionActionErrorWasm {
  inner: MissingDocumentTransitionActionError,
}

impl From<&MissingDocumentTransitionActionError> for MissingDocumentTransitionActionErrorWasm {
  fn from(e: &MissingDocumentTransitionActionError) -> Self {
    Self { inner: e.clone() }
  }
}

#[wasm_bindgen(js_class=MissingDocumentTransitionActionError)]
impl MissingDocumentTransitionActionErrorWasm {
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
