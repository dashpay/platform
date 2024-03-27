use dpp::consensus::basic::data_contract::InvalidDocumentTypeNameError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDocumentTypeNameError)]
pub struct InvalidDocumentTypeNameErrorWasm {
    inner: InvalidDocumentTypeNameError,
}

#[wasm_bindgen]
impl InvalidDocumentTypeNameErrorWasm {
    #[wasm_bindgen(js_name=getName)]
    pub fn get_name(&self) -> String {
        self.inner.name().to_string()
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

impl From<&InvalidDocumentTypeNameError> for InvalidDocumentTypeNameErrorWasm {
    fn from(e: &InvalidDocumentTypeNameError) -> Self {
        Self { inner: e.clone() }
    }
}
