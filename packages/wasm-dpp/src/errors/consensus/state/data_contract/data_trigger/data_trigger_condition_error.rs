use crate::buffer::Buffer;

use wasm_bindgen::prelude::*;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;

#[wasm_bindgen(js_name=DataTriggerConditionError)]
pub struct DataTriggerConditionErrorWasm {
  inner: DataTriggerConditionError,
}

impl From<&DataTriggerConditionError> for DataTriggerConditionErrorWasm {
  fn from(e: &DataTriggerConditionError) -> Self {
    Self { inner: e.clone() }
  }
}

#[wasm_bindgen(js_class=DataTriggerConditionError)]
impl DataTriggerConditionErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.data_contract_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.document_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getMessage)]
    pub fn message(&self) -> String {
        self.inner.message().to_string()
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

