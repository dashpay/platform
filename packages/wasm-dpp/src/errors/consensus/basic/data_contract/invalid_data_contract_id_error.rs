use crate::buffer::Buffer;
use wasm_bindgen::prelude::*;
use dpp::consensus::basic::data_contract::InvalidDataContractIdError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

#[wasm_bindgen(js_name=InvalidDataContractIdError)]
pub struct InvalidDataContractIdErrorWasm {
  inner: InvalidDataContractIdError,
}

impl From<&InvalidDataContractIdError> for InvalidDataContractIdErrorWasm {
  fn from(e: &InvalidDataContractIdError) -> Self {
    Self { inner: e.clone() }
  }
}

#[wasm_bindgen(js_class=InvalidDataContractIdError)]
impl InvalidDataContractIdErrorWasm {
    #[wasm_bindgen(js_name=getExpectedId)]
    pub fn get_expected_id(&self) -> Buffer {
        Buffer::from_bytes(&self.inner.expected_id())
    }

    #[wasm_bindgen(js_name=getInvalidId)]
    pub fn get_invalid_id(&self) -> Buffer {
        Buffer::from_bytes(&self.inner.invalid_id())
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
