use dpp::consensus::basic::data_contract::IncompatibleDocumentTypeSchemaError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IncompatibleDocumentTypeSchemaError)]
pub struct IncompatibleDocumentTypeSchemaErrorWasm {
    inner: IncompatibleDocumentTypeSchemaError,
}

impl From<&IncompatibleDocumentTypeSchemaError> for IncompatibleDocumentTypeSchemaErrorWasm {
    fn from(e: &IncompatibleDocumentTypeSchemaError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IncompatibleDocumentTypeSchemaError)]
impl IncompatibleDocumentTypeSchemaErrorWasm {
    #[wasm_bindgen(js_name=getOperation)]
    pub fn get_operation(&self) -> String {
        self.inner.operation().to_string()
    }

    #[wasm_bindgen(js_name=getPropertyPath)]
    pub fn get_property_path(&self) -> String {
        self.inner.property_path().to_string()
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
