use dpp::consensus::basic::data_contract::UniqueIndicesLimitReachedError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=UniqueIndicesLimitReachedError)]
pub struct UniqueIndicesLimitReachedErrorWasm {
    inner: UniqueIndicesLimitReachedError,
}

impl From<&UniqueIndicesLimitReachedError> for UniqueIndicesLimitReachedErrorWasm {
    fn from(e: &UniqueIndicesLimitReachedError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=UniqueIndicesLimitReachedError)]
impl UniqueIndicesLimitReachedErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.inner.document_type().to_string()
    }

    #[wasm_bindgen(js_name=getIndexLimit)]
    pub fn get_index_limit(&self) -> u16 {
        self.inner.index_limit()
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
