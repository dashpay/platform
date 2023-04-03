use wasm_bindgen::prelude::*;
use dpp::consensus::basic::data_contract::IncompatibleRe2PatternError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=IncompatibleRe2PatternError)]
pub struct IncompatibleRe2PatternErrorWasm {
  inner: IncompatibleRe2PatternError,
}

impl From<&IncompatibleRe2PatternError> for IncompatibleRe2PatternErrorWasm {
  fn from(e: &IncompatibleRe2PatternError) -> Self {
    Self { inner: e.clone() }
  }
}

#[wasm_bindgen(js_class=IncompatibleRe2PatternError)]
impl IncompatibleRe2PatternErrorWasm {
    #[wasm_bindgen(js_name=getPattern)]
    pub fn get_pattern(&self) -> String {
        self.inner.pattern().to_string()
    }

    #[wasm_bindgen(js_name=getPath)]
    pub fn get_path(&self) -> String {
        self.inner.path().to_string()
    }

    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        self.inner.message().clone()
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
