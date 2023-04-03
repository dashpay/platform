use wasm_bindgen::prelude::*;
use dpp::consensus::basic::data_contract::DataContractImmutablePropertiesUpdateError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use crate::buffer::Buffer;

#[wasm_bindgen(js_name=DataContractImmutablePropertiesUpdateError)]
pub struct DataContractImmutablePropertiesUpdateErrorWasm {
  inner: DataContractImmutablePropertiesUpdateError,
}

impl From<&DataContractImmutablePropertiesUpdateError> for DataContractImmutablePropertiesUpdateErrorWasm {
  fn from(e: &DataContractImmutablePropertiesUpdateError) -> Self {
    Self { inner: e.clone() }
  }
}

#[wasm_bindgen(js_class=DataContractImmutablePropertiesUpdateError)]
impl DataContractImmutablePropertiesUpdateErrorWasm {
    #[wasm_bindgen(js_name=getOperation)]
    pub fn get_operation(&self) -> String {
        self.inner.operation().to_string()
    }

    #[wasm_bindgen(js_name=getFieldPath)]
    pub fn get_field_path(&self) -> String {
        self.inner.field_path().to_string()
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
