use dpp::consensus::basic::data_contract::InvalidCompoundIndexError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidCompoundIndexError)]
pub struct InvalidCompoundIndexErrorWasm {
    inner: InvalidCompoundIndexError,
}

impl From<&InvalidCompoundIndexError> for InvalidCompoundIndexErrorWasm {
    fn from(e: &InvalidCompoundIndexError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidCompoundIndexError)]
impl InvalidCompoundIndexErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.inner.document_type().to_string()
    }

    #[wasm_bindgen(js_name=getIndexName)]
    pub fn get_index_name(&self) -> String {
        self.inner.index_name().to_string()
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
