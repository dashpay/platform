use dpp::consensus::basic::data_contract::InvalidIndexedPropertyConstraintError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidIndexedPropertyConstraintError)]
pub struct InvalidIndexedPropertyConstraintErrorWasm {
    inner: InvalidIndexedPropertyConstraintError,
}

impl From<&InvalidIndexedPropertyConstraintError> for InvalidIndexedPropertyConstraintErrorWasm {
    fn from(e: &InvalidIndexedPropertyConstraintError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidIndexedPropertyConstraintError)]
impl InvalidIndexedPropertyConstraintErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.inner.document_type().to_string()
    }

    #[wasm_bindgen(js_name=getIndexName)]
    pub fn get_index_name(&self) -> String {
        self.inner.index_name().to_string()
    }

    #[wasm_bindgen(js_name=getPropertyName)]
    pub fn get_property_name(&self) -> String {
        self.inner.property_name().to_string()
    }

    #[wasm_bindgen(js_name=getConstraintName)]
    pub fn get_constraint_name(&self) -> String {
        self.inner.constraint_name().to_string()
    }

    #[wasm_bindgen(js_name=getReason)]
    pub fn get_reason(&self) -> String {
        self.inner.reason().to_string()
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
