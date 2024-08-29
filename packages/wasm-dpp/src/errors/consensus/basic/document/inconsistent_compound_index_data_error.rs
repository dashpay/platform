use dpp::consensus::basic::document::InconsistentCompoundIndexDataError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use js_sys::JsString;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InconsistentCompoundIndexDataError)]
pub struct InconsistentCompoundIndexDataErrorWasm {
    inner: InconsistentCompoundIndexDataError,
}

impl From<&InconsistentCompoundIndexDataError> for InconsistentCompoundIndexDataErrorWasm {
    fn from(e: &InconsistentCompoundIndexDataError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InconsistentCompoundIndexDataError)]
impl InconsistentCompoundIndexDataErrorWasm {
    #[wasm_bindgen(js_name=getIndexedProperties)]
    pub fn get_indexed_properties(&self) -> js_sys::Array {
        self.inner
            .index_properties()
            .iter()
            .map(|string| JsString::from(string.as_ref()))
            .collect()
    }

    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.inner.document_type()
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
