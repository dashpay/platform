use wasm_bindgen::prelude::*;
use dpp::consensus::basic::state_transition::StateTransitionMaxSizeExceededError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=StateTransitionMaxSizeExceededError)]
pub struct StateTransitionMaxSizeExceededErrorWasm {
  inner: StateTransitionMaxSizeExceededError,
}

impl From<&StateTransitionMaxSizeExceededError> for StateTransitionMaxSizeExceededErrorWasm {
  fn from(e: &StateTransitionMaxSizeExceededError) -> Self {
    Self { inner: e.clone() }
  }
}

#[wasm_bindgen(js_class=StateTransitionMaxSizeExceededError)]
impl StateTransitionMaxSizeExceededErrorWasm {
    #[wasm_bindgen(js_name=getActualSizeKBytes)]
    pub fn get_actual_size_kbytes(&self) -> usize {
        self.inner.actual_size_kbytes()
    }

    #[wasm_bindgen(js_name=getMaxSizeKBytes)]
    pub fn get_max_size_kbytes(&self) -> usize {
        self.inner.max_size_kbytes()
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
