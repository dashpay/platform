use crate::buffer::Buffer;
use dpp::consensus::basic::data_contract::IncompatibleDataContractSchemaError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IncompatibleDataContractSchemaError)]
pub struct IncompatibleDataContractSchemaErrorWasm {
    inner: IncompatibleDataContractSchemaError,
}

impl From<&IncompatibleDataContractSchemaError> for IncompatibleDataContractSchemaErrorWasm {
    fn from(e: &IncompatibleDataContractSchemaError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IncompatibleDataContractSchemaError)]
impl IncompatibleDataContractSchemaErrorWasm {
    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.data_contract_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getOperation)]
    pub fn get_operation(&self) -> String {
        self.inner.operation()
    }

    #[wasm_bindgen(js_name=getFieldPath)]
    pub fn get_field_path(&self) -> String {
        self.inner.field_path()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
